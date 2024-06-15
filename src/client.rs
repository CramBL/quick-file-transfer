use std::{
    fs,
    io::{self, BufWriter, Write},
    net::TcpStream,
};

use crate::{
    config::{self, Config},
    util::tcp_stream,
};
use anyhow::{Ok, Result};

const BUFSIZE_READER: usize = 32 * 1024;

pub fn run_client(cfg: Config) -> Result<()> {
    let mut tcp_stream = tcp_stream(cfg.address())?;

    log::info!("Connection to: {}", cfg.address());
    if let Some(msg) = cfg.message() {
        let res = tcp_stream.write_all(msg.as_bytes());
        log::debug!("Wrote message: {msg}");
        log::debug!("TCP write result: {res:?}");
    }

    if let Some(file) = cfg.file() {
        log::info!(
            "Sending {file:?} with compression={}",
            cfg.compression().unwrap_or_default()
        );
        match cfg.compression().unwrap_or_default() {
            config::Compression::Lz4 => {
                //
                use lz4_flex::frame::FrameEncoder;
                let f = fs::File::open(file)?;
                let mut bufreader = io::BufReader::with_capacity(BUFSIZE_READER, f);
                let mut lz4_writer = FrameEncoder::new(&tcp_stream);
                io::copy(&mut bufreader, &mut lz4_writer).expect("I/O operation failed");
                lz4_writer.finish()?;
            }
            config::Compression::Gzip => todo!(),
            config::Compression::Bzip2 => todo!(),
            config::Compression::Xz => todo!(),
            config::Compression::None => match cfg.file() {
                Some(p) => {
                    let f = fs::File::open(p)?;
                    let mut bufreader = io::BufReader::with_capacity(BUFSIZE_READER, f);
                    let mut bufwriter = BufWriter::with_capacity(BUFSIZE_READER, &tcp_stream);
                    let len = io::copy(&mut bufreader, &mut bufwriter)?;
                    log::info!("Wrote {len} to stream");
                }
                None => todo!(),
            },
        }
    }

    tcp_stream.shutdown(std::net::Shutdown::Write)?;
    Ok(())
}
