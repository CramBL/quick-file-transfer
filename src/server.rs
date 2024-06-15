use lz4_flex::frame::FrameDecoder;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdoutLock},
    net::TcpStream,
    path::Path,
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

            match cfg.compression().unwrap_or_default() {
                config::Compression::Lz4 => {
                    let mut reader = FrameDecoder::new(&socket);
                    let len = io::copy(&mut reader, bufwriter)?;
                    log::info!("Received: {len}");
                }
                config::Compression::Gzip => todo!("Not implemented"),
                config::Compression::Bzip2 => todo!("Not implemented"),
                config::Compression::Xz => todo!("Not implemented"),
                config::Compression::None => {
                    let len = copy_all_from_tcp_stream(socket, bufwriter)?;
                    log::info!("Received: {len} bytes");
                }
            }
        }
        Err(e) => println!("couldn't get client: {e:?}"),
    }

    log::info!("Server closing...");
    Ok(())
}

pub fn file_with_bufwriter(path: &Path) -> Result<BufWriter<File>> {
    let f = fs::File::create(path)?;
    let writer = BufWriter::with_capacity(BUFSIZE_FILE_WRITE, f);
    Ok(writer)
}

pub fn stdout_bufwriter() -> BufWriter<StdoutLock<'static>> {
    let stdout = io::stdout().lock();
    let writer = BufWriter::with_capacity(BUFSIZE_FILE_WRITE, stdout);
    writer
}

pub fn tcp_bufreader(socket: TcpStream) -> BufReader<TcpStream> {
    BufReader::with_capacity(BUFSIZE_FILE_WRITE, socket)
}

pub fn copy_all_from_tcp_stream(socket: TcpStream, writer: &mut dyn io::Write) -> Result<u64> {
    let mut reader = tcp_bufreader(socket);
    let len = io::copy(&mut reader, writer)?;
    Ok(len)
}
