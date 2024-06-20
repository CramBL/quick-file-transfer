use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, StdoutLock},
    net::TcpListener,
    path::Path,
};

use crate::{
    config::{self, transfer::ContentTransferArgs, Config},
    util::{create_file_with_len, format_data_size, incremental_rw},
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::Result;

pub fn run_server(
    ip: &str,
    port: u16,
    _cfg: &Config,
    content_transfer_args: &ContentTransferArgs,
) -> Result<()> {
    let socket_addr = (ip, port);
    let listener = TcpListener::bind(socket_addr)?;

    log::info!(
        "Listening on: {ip}:{port} for a {}",
        match content_transfer_args.compression() {
            Some(c) => {
                match c {
                    config::compression::Compression::Bzip2(_) => "bzip2 compressed file",
                    config::compression::Compression::Gzip(_) => "gzip compressed file",
                    config::compression::Compression::Lz4 => "lz4 compressed file",
                    config::compression::Compression::Xz(_) => "xz compressed file",
                }
            }
            None => "raw file",
        }
    );
    // On-stack dynamic dispatch
    let (mut stdout_write, mut file_write);
    let bufwriter: &mut dyn io::Write = match content_transfer_args.file() {
        Some(p) => {
            file_write = file_with_bufwriter(p)?;
            &mut file_write
        }
        None => {
            stdout_write = stdout_bufwriter();
            &mut stdout_write
        }
    };

    match listener.accept() {
        Ok((mut socket, addr)) => {
            log::info!("Client accepted at: {addr:?}");
            if content_transfer_args.prealloc() {
                let mut size_buffer = [0u8; 8];
                socket.read_exact(&mut size_buffer)?;
                let file_size = u64::from_be_bytes(size_buffer);
                log::debug!(
                    "Preallocating file of size {} [{file_size} B]",
                    format_data_size(file_size)
                );
                create_file_with_len(content_transfer_args.file().unwrap(), file_size)?;
            }
            let mut buf_tcp_reader = BufReader::with_capacity(BUFFERED_RW_BUFSIZE, socket);

            let len = match content_transfer_args.compression() {
                Some(compr) => match compr {
                    config::compression::Compression::Bzip2(_) => {
                        let mut tcp_decoder = bzip2::read::BzDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                    config::compression::Compression::Gzip(_) => {
                        let mut tcp_decoder = GzDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                    config::compression::Compression::Lz4 => {
                        let mut tcp_decoder = FrameDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                    config::compression::Compression::Xz(_) => {
                        let mut tcp_decoder = xz2::read::XzDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                },
                None => incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut buf_tcp_reader)?,
            };
            log::info!("Received: {} [{len} B]", format_data_size(len));
        }
        Err(e) => println!("Failed accepting connection to client: {e:?}"),
    }

    log::debug!("Server exiting...");
    Ok(())
}

pub fn file_with_bufwriter(path: &Path) -> Result<BufWriter<File>> {
    let f = fs::File::create(path)?;
    let writer = BufWriter::with_capacity(BUFFERED_RW_BUFSIZE, f);
    Ok(writer)
}

pub fn stdout_bufwriter() -> BufWriter<StdoutLock<'static>> {
    let stdout = io::stdout().lock();
    BufWriter::with_capacity(BUFFERED_RW_BUFSIZE, stdout)
}
