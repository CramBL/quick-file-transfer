use anyhow::{bail, Result};
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, StdinLock, Write},
    net::{TcpStream, ToSocketAddrs},
    path::Path,
};

use crate::{
    config::transfer::{
        command::ServerCommand,
        util::{PollAbortCondition, TcpConnectMode},
    },
    util::tiny_rnd::rnd_u32,
    BUFFERED_RW_BUFSIZE,
};

pub fn file_with_bufreader(path: &Path) -> Result<BufReader<File>> {
    let f = fs::File::open(path)?;
    let reader = BufReader::with_capacity(BUFFERED_RW_BUFSIZE, f);
    Ok(reader)
}

pub fn stdin_bufreader() -> BufReader<StdinLock<'static>> {
    let stdin = io::stdin().lock();
    BufReader::with_capacity(BUFFERED_RW_BUFSIZE, stdin)
}

pub fn tcp_bufwriter(socket: &TcpStream) -> BufWriter<&TcpStream> {
    BufWriter::with_capacity(BUFFERED_RW_BUFSIZE, socket)
}

/// Send a [ServerCommand] to the server
pub fn send_command(stream: &mut TcpStream, command: &ServerCommand) -> anyhow::Result<()> {
    log::trace!("Sending command: {command:?}");
    let command_bytes = bincode::serialize(command)?;
    debug_assert!(command_bytes.len() <= u8::MAX as usize);
    let size = command_bytes.len() as u8;
    let header = size.to_be_bytes();

    // Send the header followed by the command
    stream.write_all(&header)?;
    stream.write_all(&command_bytes)?;
    Ok(())
}

/// Perform the simple QFT handshake from the client end.
///
/// The handshake is simply to ensure why are talking to a QFT server
fn qft_client_handshake(socket: &mut TcpStream) -> anyhow::Result<()> {
    let mut handshake_buf: [u8; 4] = [0; 4];
    socket.read_exact(&mut handshake_buf)?;
    let recv_handshake: u32 = u32::from_be_bytes(handshake_buf);
    let return_handshake: u32 = rnd_u32(recv_handshake as u64);
    socket.write_all(&return_handshake.to_be_bytes())?;
    Ok(())
}

/// Connect to a QFT server
pub fn qft_connect_to_server<A>(
    socket_addr: A,
    connect_mode: TcpConnectMode,
) -> anyhow::Result<TcpStream>
where
    A: ToSocketAddrs,
{
    match connect_mode {
        TcpConnectMode::OneShot => {
            let mut socket = TcpStream::connect(socket_addr)?;
            qft_client_handshake(&mut socket)?;
            Ok(socket)
        }
        TcpConnectMode::Poll(poll_opts) => {
            let mut attempts: u32 = 0;
            let now = std::time::Instant::now();
            loop {
                match TcpStream::connect(&socket_addr) {
                    Ok(mut socket) => {
                        if let Err(e) = qft_client_handshake(&mut socket) {
                            log::warn!("Handshake failed: {e} ... retrying");
                        } else {
                            break Ok(socket);
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
    }
}
