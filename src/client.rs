use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, StdinLock, Write},
    net::TcpStream,
    path::Path,
};

use crate::{
    config::{self, Config},
    mmap_reader::MemoryMappedReader,
    util::{format_data_size, tcp_stream},
    TCP_STREAM_BUFSIZE,
};
use anyhow::Result;
use flate2::{
    bufread::{self, GzEncoder},
    Compression,
};

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
    let (mut stdin_read, mut file_read, mut mmap_read);
    let bufreader: &mut dyn io::BufRead = match cfg.file() {
        Some(p) if cfg.use_mmap() => {
            let ammap_read = MemoryMappedReader::new(p)?;
            mmap_read = BufReader::new(ammap_read);
            &mut mmap_read
        }
        Some(p) => {
            file_read = file_with_bufreader(p)?;
            &mut file_read
        }
        None => {
            stdin_read = stdin_bufreader();
            &mut stdin_read
        }
    };

    let transferred_bytes = match cfg.compression().unwrap_or_default() {
        config::Compression::Lz4 => {
            lz4_incremental_write_to_tcp_stream::<TCP_STREAM_BUFSIZE>(&tcp_stream, bufreader)?
        }
        config::Compression::Gzip => {
            use flate2::Compression;
            let mut encoder = GzEncoder::new(bufreader, Compression::fast());

            let mut data = [0; TCP_STREAM_BUFSIZE];
            let mut total_read = 0;
            loop {
                let bytes_read = encoder.read(&mut data)?;
                if bytes_read == 0 {
                    break;
                }
                total_read += bytes_read;
                tcp_stream.write_all(&mut data[0..bytes_read])?;
            }
            total_read as u64
        }
        config::Compression::Bzip2 => todo!(),
        config::Compression::Xz => todo!(),
        config::Compression::None => copy_all_to_tcp_stream(&tcp_stream, bufreader)?,
    };
    log::info!("Wrote {} to stream", format_data_size(transferred_bytes));

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

pub fn copy_all_to_tcp_stream(socket: &TcpStream, reader: &mut dyn io::BufRead) -> Result<u64> {
    let mut writer = tcp_bufwriter(socket);
    let len = io::copy(reader, &mut writer)?;
    Ok(len)
}

pub fn lz4_copy_all_to_tcp_stream(socket: &TcpStream, reader: &mut dyn io::BufRead) -> Result<u64> {
    use lz4_flex::frame::FrameEncoder;
    let mut lz4_writer = FrameEncoder::new(socket);
    let len = io::copy(reader, &mut lz4_writer)?;
    lz4_writer.finish()?;
    Ok(len)
}

pub fn lz4_incremental_write_to_tcp_stream<const BUFSIZE: usize>(
    socket: &TcpStream,
    reader: &mut dyn io::BufRead,
) -> Result<u64> {
    use lz4_flex::frame::FrameEncoder;
    let mut lz4_writer = FrameEncoder::new(socket);

    let mut buf = [0; BUFSIZE];
    let mut total_read = 0;
    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        total_read += bytes_read;

        lz4_writer.write(&buf[..bytes_read])?;
    }
    lz4_writer.finish()?;
    Ok(total_read as u64)
}
