use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdinLock, Write},
    net::TcpStream,
    path::Path,
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

    // On-stack dynamic dispatch
    let (mut stdin_read, mut file_read);
    let bufreader: &mut dyn io::Read = match cfg.file() {
        Some(p) => {
            file_read = file_with_bufreader(p)?;
            &mut file_read
        }
        None => {
            stdin_read = stdin_bufreader();
            &mut stdin_read
        }
    };

    if let Some(file) = cfg.file() {
        log::info!(
            "Sending {file:?} with compression={}",
            cfg.compression().unwrap_or_default()
        );
        match cfg.compression().unwrap_or_default() {
            config::Compression::Lz4 => {
                //
                use lz4_flex::frame::FrameEncoder;
                let mut lz4_writer = FrameEncoder::new(&tcp_stream);
                io::copy(bufreader, &mut lz4_writer).expect("I/O operation failed");
                lz4_writer.finish()?;
            }
            config::Compression::Gzip => todo!(),
            config::Compression::Bzip2 => todo!(),
            config::Compression::Xz => todo!(),
            config::Compression::None => {
                let len = copy_all_to_tcp_stream(&tcp_stream, bufreader)?;
                log::info!("Wrote {len} to stream");
            }
        }
    }

    tcp_stream.shutdown(std::net::Shutdown::Write)?;
    Ok(())
}

pub fn file_with_bufreader(path: &Path) -> Result<BufReader<File>> {
    let f = fs::File::open(path)?;
    let reader = BufReader::with_capacity(BUFSIZE_READER, f);
    Ok(reader)
}

pub fn stdin_bufreader() -> BufReader<StdinLock<'static>> {
    let stdin = io::stdin().lock();
    let reader = BufReader::with_capacity(BUFSIZE_READER, stdin);
    reader
}

pub fn tcp_bufwriter(socket: &TcpStream) -> BufWriter<&TcpStream> {
    BufWriter::with_capacity(BUFSIZE_READER, socket)
}

pub fn copy_all_to_tcp_stream(socket: &TcpStream, reader: &mut dyn io::Read) -> Result<u64> {
    let mut writer = tcp_bufwriter(socket);
    let len = io::copy(reader, &mut writer)?;
    Ok(len)
}
