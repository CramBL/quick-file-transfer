use std::{
    fs::File,
    io::{Read, Write},
    net::{IpAddr, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use anyhow::bail;

use crate::{
    config::{
        self,
        compression::{Bzip2Args, Compression, GzipArgs, XzArgs},
        transfer::{
            command::{DestinationMode, ServerCommand, ServerResult},
            util::TcpConnectMode,
        },
    },
    mmap_reader::MemoryMapWrapper,
    send::util::{file_with_bufreader, qft_connect_to_server, send_command, tcp_bufwriter},
    util::{format_data_size, incremental_rw, read_server_response},
    TCP_STREAM_BUFSIZE,
};

/// If poll is specified, poll the server with the specified interval, else exut on the first failure to establish a connection.
#[allow(clippy::too_many_arguments)]
pub fn run_client(
    ip: IpAddr,
    port: u16,
    use_mmap: bool,
    input_files: &[PathBuf],
    prealloc: bool,
    compression: Option<Compression>,
    connect_mode: TcpConnectMode,
    remote_dest: Option<&Path>,
) -> anyhow::Result<()> {
    let mut initial_tcp_stream = qft_connect_to_server((ip, port), connect_mode)?;

    // Validate remote path before start
    if let Some(remote_dest) = remote_dest {
        tracing::info!("Running client in remote mode targeting destination: {remote_dest:?}");
        if input_files.is_empty() {
            bail!("Error: no files to send");
        }
        let dest_mode: DestinationMode = if input_files.len() == 1 {
            DestinationMode::SingleFile
        } else {
            DestinationMode::MultipleFiles
        };

        send_command(
            &mut initial_tcp_stream,
            &ServerCommand::IsDestinationValid(
                dest_mode,
                remote_dest.to_string_lossy().into_owned(),
            ),
        )?;

        match read_server_response(&mut initial_tcp_stream)? {
            ServerResult::Ok => log::trace!("Remote path is valid"),
            ServerResult::Err(e) => {
                bail!(e);
            }
        }
    }

    let cmd_free_port = ServerCommand::GetFreePort((None, None));
    send_command(&mut initial_tcp_stream, &cmd_free_port)?;
    let mut free_port_buf: [u8; 2] = [0; 2];
    if let Err(e) = initial_tcp_stream.read_exact(&mut free_port_buf) {
        log::trace!("Initial tcp read of free port response failed: {e}, retrying in 100 ms...");
        thread::sleep(Duration::from_millis(100));
        initial_tcp_stream.read_exact(&mut free_port_buf)?;
    }
    let free_port = u16::from_be_bytes(free_port_buf);
    tracing::info!("Got free port: {free_port}");

    if input_files.is_empty() {
        let mut tcp_stream = qft_connect_to_server((ip, free_port), connect_mode)?;
        let cmd_receive_data =
            ServerCommand::ReceiveData(0, "stdin".to_string(), compression.map(|c| c.variant()));
        send_command(&mut tcp_stream, &cmd_receive_data)?;
        let transferred_len =
            transfer_data((ip, port), &mut tcp_stream, compression, None, use_mmap)?;
        log::info!(
            "Sent {} [{transferred_len} B]",
            format_data_size(transferred_len)
        );
    } else {
        let mut fcount = input_files.len();
        log::info!("Sending {fcount} file(s)");

        for f in input_files {
            let mut tcp_stream = qft_connect_to_server((ip, free_port), connect_mode)?;

            fcount -= 1;
            let fname: String = f.file_name().unwrap().to_str().unwrap().to_owned();
            if prealloc {
                let file_size = File::open(f)?.metadata()?.len();
                tracing::debug!(
                    "Requesting preallocation of file of size {} [{file_size} B]",
                    format_data_size(file_size)
                );
                send_command(
                    &mut tcp_stream,
                    &ServerCommand::Prealloc(
                        file_size,
                        f.file_name().unwrap().to_string_lossy().into(),
                    ),
                )?;
            }

            log::trace!("Sending receive data command");
            let cmd_receive_data =
                ServerCommand::ReceiveData(fcount as u32, fname, compression.map(|c| c.variant()));
            send_command(&mut tcp_stream, &cmd_receive_data)?;

            let transferred_len =
                transfer_data((ip, port), &mut tcp_stream, compression, Some(f), use_mmap)?;
            tcp_stream.flush()?;

            log::info!(
                "Sent {file} {} [{transferred_len} B]",
                format_data_size(transferred_len),
                file = f.display()
            );
        }
    }

    send_command(&mut initial_tcp_stream, &ServerCommand::EndOfTransfer)?;
    query_server_result(&mut initial_tcp_stream)?;

    Ok(())
}

pub fn query_server_result(initial_tcp_stream: &mut TcpStream) -> anyhow::Result<()> {
    use config::transfer::command::ServerResult;
    let mut header_buf = [0; ServerResult::HEADER_SIZE];
    // Read the header to determine the size of the incoming command/data
    if let Err(e) = initial_tcp_stream.read_exact(&mut header_buf) {
        log::warn!("{}: {e}, retrying in 100 ms ...", e.kind());
        std::thread::sleep(Duration::from_millis(100));
        initial_tcp_stream.read_exact(&mut header_buf)?;
    }
    let inc_cmd_len = ServerResult::size_from_bytes(header_buf);

    let mut resp_buf = vec![0; inc_cmd_len];

    // Read the actual command/data based on the size
    if let Err(e) = initial_tcp_stream.read_exact(&mut resp_buf[..inc_cmd_len]) {
        anyhow::bail!("Error reading command into buffer: {e}");
    }
    let resp: ServerResult = bincode::deserialize(&resp_buf[..inc_cmd_len])?;
    log::debug!("Server response: {resp:?}");

    match resp {
        ServerResult::Ok => Ok(()),
        ServerResult::Err(err_str) => bail!("Server responded with an error: {err_str}"),
    }
}

fn transfer_data(
    (ip, port): (IpAddr, u16),
    tcp_stream: &mut TcpStream,
    compression: Option<Compression>,
    file: Option<&Path>,
    use_mmap: bool,
) -> anyhow::Result<u64> {
    log::debug!("Sending to: {ip}:{port}");

    let mut buf_tcp_stream = tcp_bufwriter(tcp_stream);

    if use_mmap && file.is_some() {
        log::debug!("Using mmap");
        let mmap = MemoryMapWrapper::new(file.unwrap())?;
        let target_read = mmap.flen();

        let transferred_bytes = match compression {
            None => {
                let mut total_written = 0;
                let chunks = mmap.borrow_full().chunks(TCP_STREAM_BUFSIZE);
                for chunk in chunks {
                    let mut chunk_written = 0;
                    let chunk_len = chunk.len();
                    while chunk_written != chunk_len {
                        let bytes_written = buf_tcp_stream.write(chunk)?;
                        if bytes_written == 0 {
                            bail!("Wrote 0 bytes to socket, server disconnected?");
                        }
                        chunk_written += bytes_written;
                    }
                    total_written += chunk_written;
                }

                total_written.try_into()?
            }
            Some(c) => match c {
                config::compression::Compression::Bzip2(Bzip2Args { compression_level }) => {
                    let mut encoder = bzip2::read::BzEncoder::new(
                        mmap.borrow_full(),
                        bzip2::Compression::new(compression_level.into()),
                    );
                    incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut buf_tcp_stream, &mut encoder)?
                }
                config::compression::Compression::Lz4 => {
                    let mut lz4_writer = lz4_flex::frame::FrameEncoder::new(&mut buf_tcp_stream);
                    let mut total_read = 0;
                    while total_read < target_read {
                        let remaining = target_read - total_read;
                        let chunk_size = remaining.min(TCP_STREAM_BUFSIZE);
                        let chunk = mmap.borrow_slice(total_read..total_read + chunk_size)?;
                        let written_bytes = lz4_writer.write(chunk)?;
                        total_read += written_bytes;
                    }
                    lz4_writer.flush()?; // Needed to ensure the entire content is written
                    total_read as u64
                }
                config::compression::Compression::Gzip(GzipArgs { compression_level }) => {
                    let mut encoder = flate2::read::GzEncoder::new(
                        mmap.borrow_full(),
                        flate2::Compression::new(compression_level.into()),
                    );
                    incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut buf_tcp_stream, &mut encoder)?
                }
                config::compression::Compression::Xz(XzArgs { compression_level }) => {
                    let mut compressor =
                        xz2::read::XzEncoder::new(mmap.borrow_full(), compression_level.into());
                    incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(
                        &mut buf_tcp_stream,
                        &mut compressor,
                    )?
                }
            },
        };
        return Ok(transferred_bytes);
    }

    // On-stack dynamic dispatch
    let mut bufreader = file_with_bufreader(file.unwrap())?;
    if let Some(compression) = compression {
        log::debug!("Compression mode: {compression}");
    };

    let transferred_bytes = match compression {
        Some(compression) => match compression {
            config::compression::Compression::Bzip2(Bzip2Args { compression_level }) => {
                let mut encoder = bzip2::read::BzEncoder::new(
                    bufreader,
                    bzip2::Compression::new(compression_level.into()),
                );
                incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Lz4 => {
                let mut lz4_writer = lz4_flex::frame::FrameEncoder::new(&mut buf_tcp_stream);
                let len: u64 =
                    incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut lz4_writer, &mut bufreader)?;
                lz4_writer.flush()?; // Needed to ensure the entire content is written
                len
            }
            config::compression::Compression::Gzip(GzipArgs { compression_level }) => {
                let mut encoder = flate2::read::GzEncoder::new(
                    bufreader,
                    flate2::Compression::new(compression_level.into()),
                );
                incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Xz(XzArgs { compression_level }) => {
                let mut compressor = xz2::read::XzEncoder::new(bufreader, compression_level.into());
                incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut buf_tcp_stream, &mut compressor)?
            }
        },
        None => incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut buf_tcp_stream, &mut bufreader)?,
    };

    Ok(transferred_bytes)
}
