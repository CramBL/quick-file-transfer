use std::{
    fs::File,
    io::{self, Read, Write},
    net::{IpAddr, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use crate::{
    config::{
        self,
        compression::{Bzip2Args, Compression, GzipArgs, XzArgs},
        transfer::{command::ServerCommand, util::TcpConnectMode},
    },
    mmap_reader::MemoryMappedReader,
    send::util::{
        file_with_bufreader, qft_connect_to_server, send_command, stdin_bufreader, tcp_bufwriter,
    },
    util::{format_data_size, incremental_rw},
    TCP_STREAM_BUFSIZE,
};

/// If poll is specified, poll the server with the specified interval, else exut on the first failure to establish a connection.
pub fn run_client(
    ip: IpAddr,
    port: u16,
    use_mmap: bool,
    input_files: &[PathBuf],
    prealloc: bool,
    compression: Option<Compression>,
    connect_mode: TcpConnectMode,
) -> anyhow::Result<()> {
    let mut initial_tcp_stream = qft_connect_to_server((ip, port), connect_mode)?;

    let cmd_free_port = ServerCommand::GetFreePort((None, None));
    send_command(&mut initial_tcp_stream, &cmd_free_port)?;
    let mut free_port_buf: [u8; 2] = [0; 2];
    if let Err(e) = initial_tcp_stream.read_exact(&mut free_port_buf) {
        log::trace!("Initial tcp read of free port response failed: {e}, retrying in 100 ms...");
        thread::sleep(Duration::from_millis(100));
        initial_tcp_stream.read_exact(&mut free_port_buf)?;
    }
    let free_port = u16::from_be_bytes(free_port_buf);
    log::info!("Got free port: {free_port}");

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
        log::info!("Sending {fcount} files");
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
                "Sent {} [{transferred_len} B]",
                format_data_size(transferred_len)
            );
        }
    }

    Ok(())
}

fn transfer_data(
    (ip, port): (IpAddr, u16),
    tcp_stream: &mut TcpStream,
    compression: Option<Compression>,
    file: Option<&Path>,
    use_mmap: bool,
) -> anyhow::Result<u64> {
    log::info!("Connecting to: {ip}:{port}");

    let mut buf_tcp_stream = tcp_bufwriter(tcp_stream);

    // On-stack dynamic dispatch
    let (mut stdin_read, mut file_read, mut mmap_read);
    let bufreader: &mut dyn io::Read = match file {
        Some(p) if use_mmap => {
            log::debug!("Opening file in memory map mode");
            mmap_read = MemoryMappedReader::new(p)?;
            &mut mmap_read
        }
        Some(p) => {
            log::debug!("Opening file in buffered reading mode");
            file_read = file_with_bufreader(p)?;
            &mut file_read
        }
        None => {
            log::debug!("Reading from stdin");
            stdin_read = stdin_bufreader();
            &mut stdin_read
        }
    };

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
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Lz4 => {
                let mut lz4_writer = lz4_flex::frame::FrameEncoder::new(&mut buf_tcp_stream);
                let len = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut lz4_writer, bufreader)?;
                lz4_writer.flush()?; // Needed to ensure the entire content is written
                len
            }
            config::compression::Compression::Gzip(GzipArgs { compression_level }) => {
                let mut encoder = flate2::read::GzEncoder::new(
                    bufreader,
                    flate2::Compression::new(compression_level.into()),
                );
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Xz(XzArgs { compression_level }) => {
                let mut compressor = xz2::read::XzEncoder::new(bufreader, compression_level.into());
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut compressor)?
            }
        },
        None => incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, bufreader)?,
    };

    Ok(transferred_bytes)
}
