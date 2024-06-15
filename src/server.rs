use std::{
    fs,
    io::{self, BufReader, BufWriter, Read, Write},
    net::TcpStream,
};

use crate::{
    config::{self, Config},
    util::tcp_listen,
};
use anyhow::Result;

const BUFSIZE_FILE_WRITE: usize = 32 * 1024;

pub fn run_server(cfg: Config) -> Result<()> {
    let listener = tcp_listen(cfg.address())?;

    log::info!("Listening on: {}", cfg.address());
    match listener.accept() {
        Ok((socket, addr)) => {
            println!("new client: {addr:?}");

            match cfg.compression().unwrap_or_default() {
                config::Compression::Lz4 => {
                    use lz4_flex::frame::FrameDecoder;
                    let f = std::fs::File::create(cfg.file().unwrap())?;
                    let mut writer = BufWriter::with_capacity(BUFSIZE_FILE_WRITE, f);
                    let mut reader = FrameDecoder::new(&socket);
                    let len = io::copy(&mut reader, &mut writer)?;
                    writer.flush()?;
                    log::info!("Wrote: {len}");
                }
                config::Compression::Gzip => todo!(),
                config::Compression::Bzip2 => todo!(),
                config::Compression::Xz => todo!(),
                config::Compression::None => match cfg.file() {
                    Some(p) => {
                        let f = fs::File::create(p)?;
                        let mut reader = BufReader::with_capacity(BUFSIZE_FILE_WRITE, &socket);
                        let mut writer = BufWriter::with_capacity(BUFSIZE_FILE_WRITE, f);
                        let len = io::copy(&mut reader, &mut writer)?;
                        log::info!("Received: {len} bytes");
                    }
                    None => {
                        let stdout = io::stdout().lock();
                        let mut reader = BufReader::with_capacity(BUFSIZE_FILE_WRITE, &socket);
                        let mut writer = BufWriter::with_capacity(BUFSIZE_FILE_WRITE, stdout);
                        let len = io::copy(&mut reader, &mut writer)?;
                        log::info!("Received: {len} bytes");
                    }
                },
            }
        }
        Err(e) => println!("couldn't get client: {e:?}"),
    }

    log::info!("Server closing...");
    Ok(())
}
