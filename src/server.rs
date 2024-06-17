use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, StdoutLock},
    path::Path,
};

use crate::{
    config::{self, Config},
    util::{bind_tcp_listener, create_file_with_len, format_data_size, incremental_rw, Address},
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::Result;

pub fn run_server(ip: &str, cfg: &Config) -> Result<()> {
    let addr = Address::new(ip, cfg.port().unwrap());
    let listener = bind_tcp_listener(addr)?;

    log::info!("Listening on: {addr}");
    // On-stack dynamic dispatch
    let (mut stdout_write, mut file_write);
    let bufwriter: &mut dyn io::Write = match cfg.file() {
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
            if cfg.prealloc() {
                let mut size_buffer = [0u8; 8];
                socket.read_exact(&mut size_buffer)?;
                let file_size = u64::from_be_bytes(size_buffer);
                log::debug!(
                    "Preallocating file of size {} [{file_size} B]",
                    format_data_size(file_size)
                );
                create_file_with_len(cfg.file().unwrap(), file_size)?;
            }
            let mut buf_tcp_reader = BufReader::with_capacity(BUFFERED_RW_BUFSIZE, socket);

            let len = match cfg.compression().unwrap_or_default() {
                config::Compression::Lz4 => {
                    let mut tcp_decoder = FrameDecoder::new(buf_tcp_reader);
                    incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                }
                config::Compression::Gzip => {
                    let mut tcp_decoder = GzDecoder::new(buf_tcp_reader);
                    incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                }
                config::Compression::Bzip2 => todo!("Not implemented"),
                config::Compression::Xz => todo!("Not implemented"),
                config::Compression::None => {
                    incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut buf_tcp_reader)?
                }
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
