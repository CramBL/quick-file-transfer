use std::{
    fs::File,
    io::{self, Read, Write},
    net::{IpAddr, TcpStream},
    path::Path,
};

use crate::{
    config::{
        self,
        compression::{Bzip2Args, Compression, GzipArgs, XzArgs},
        transfer::{
            command::ServerCommand,
            util::{PollAbortCondition, TcpConnectMode},
        },
    },
    mmap_reader::MemoryMappedReader,
    send::util::{file_with_bufreader, stdin_bufreader, tcp_bufwriter},
    util::{format_data_size, incremental_rw, tiny_rnd::rnd_u32},
    TCP_STREAM_BUFSIZE,
};

use anyhow::{bail, Result};

fn send_command(stream: &mut TcpStream, command: &ServerCommand) -> anyhow::Result<()> {
    let command_bytes = bincode::serialize(command)?;
    debug_assert!(command_bytes.len() <= u8::MAX as usize);
    let size = command_bytes.len() as u8;
    let header = size.to_be_bytes();

    // Send the header followed by the command
    stream.write_all(&header)?;
    stream.write_all(&command_bytes)?;
    Ok(())
}

fn qft_client_handshake(socket: &mut TcpStream) -> anyhow::Result<()> {
    let mut handshake_buf: [u8; 4] = [0; 4];
    socket.read_exact(&mut handshake_buf)?;
    let recv_handshake: u32 = u32::from_be_bytes(handshake_buf);
    let return_handshake: u32 = rnd_u32(recv_handshake as u64);
    socket.write_all(&return_handshake.to_be_bytes())?;
    Ok(())
}

/// If poll is specified, poll the server with the specified interval, else exut on the first failure to establish a connection.
pub fn run_client(
    ip: IpAddr,
    port: u16,
    use_mmap: bool,
    input_file: Option<&Path>,
    prealloc: bool,
    compression: Option<Compression>,
    connect_mode: TcpConnectMode,
) -> Result<()> {
    let socket_addr = (ip, port);

    let mut tcp_stream = match connect_mode {
        TcpConnectMode::OneShot => {
            let mut socket = TcpStream::connect(socket_addr)?;
            qft_client_handshake(&mut socket)?;
            socket
        }
        TcpConnectMode::Poll(poll_opts) => {
            let mut attempts: u32 = 0;
            let now = std::time::Instant::now();
            loop {
                match TcpStream::connect(socket_addr) {
                    Ok(mut socket) => {
                        if let Err(e) = qft_client_handshake(&mut socket) {
                            log::warn!("Handshake failed: {e} ... retrying");
                        } else {
                            break socket;
                        }
                    }
                    Err(e) => {
                        log::trace!("Connection attempt failed: {e}");
                        match e.kind() {
                            io::ErrorKind::NotFound
                            | io::ErrorKind::ConnectionRefused
                            | io::ErrorKind::ConnectionReset
                            | io::ErrorKind::NotConnected
                            | io::ErrorKind::BrokenPipe
                            | io::ErrorKind::TimedOut
                            | io::ErrorKind::Interrupted => {
                                log::debug!("Retrying TCP connection in {:?}", poll_opts.interval)
                            }
                            _ => bail!(e),
                        }
                    }
                }
                match poll_opts.abort_condition {
                    PollAbortCondition::Attempts(att) => {
                        attempts += 1;
                        if attempts == att {
                            bail!("Failed establishing a TCP connection after {att} attempts")
                        }
                    }
                    PollAbortCondition::Timeout(timeout_dur) => {
                        if now.elapsed() >= timeout_dur {
                            bail!("Failed establishing a TCP connection after {timeout_dur:?}")
                        }
                    }
                };
                std::thread::sleep(poll_opts.interval);
            }
        }
    };

    if prealloc {
        let file_size = File::open(input_file.unwrap())?.metadata()?.len();
        log::debug!(
            "Requesting preallocation of file of size {} [{file_size} B]",
            format_data_size(file_size)
        );
        let cmd_prealloc = ServerCommand::Prealloc(file_size);
        log::debug!("Sending command: {cmd_prealloc:?}");
        send_command(&mut tcp_stream, &cmd_prealloc)?;
    }

    let cmd_receive_data = ServerCommand::ReceiveData(compression.map(|c| c.variant()));
    send_command(&mut tcp_stream, &cmd_receive_data)?;
    let transferred_len =
        transfer_data((ip, port), &tcp_stream, compression, input_file, use_mmap)?;

    log::info!(
        "Sent {} [{transferred_len} B]",
        format_data_size(transferred_len)
    );

    Ok(())
}

fn transfer_data(
    (ip, port): (IpAddr, u16),
    tcp_stream: &TcpStream,
    compression: Option<Compression>,
    file: Option<&Path>,
    use_mmap: bool,
) -> Result<u64> {
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
