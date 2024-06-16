use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdoutLock},
    path::Path,
};

use crate::{
    config::{self, Config},
    util::{bind_tcp_listener, format_data_size, incremental_rw},
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::Result;

pub fn run_server(cfg: &Config) -> Result<()> {
    let listener = bind_tcp_listener(cfg.address())?;

    log::info!("Listening on: {}", cfg.address());
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
        Ok((socket, addr)) => {
            println!("new client: {addr:?}");
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
            log::info!("Received: {}", format_data_size(len));
        }
        Err(e) => println!("Failed accepting connection to client: {e:?}"),
    }

    log::info!("Server closing...");
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
