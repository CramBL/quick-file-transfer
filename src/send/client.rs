use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdinLock, Write},
    net::{IpAddr, TcpStream},
    path::Path,
};

use crate::{
    config::{self, Config},
    mmap_reader::MemoryMappedReader,
    util::{format_data_size, incremental_rw},
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::Result;
use flate2::{read::GzEncoder, Compression};

pub fn run_client(ip: IpAddr, port: u16, cfg: &Config) -> Result<()> {
    let socket_addr = (ip, port);
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    if cfg.prealloc() {
        let file_size = File::open(cfg.file().unwrap())?.metadata()?.len();
        log::debug!(
            "Requesting preallocation of file of size {} [{file_size} B]",
            format_data_size(file_size)
        );
        tcp_stream.write_all(&file_size.to_be_bytes())?;
    }
    let mut buf_tcp_stream = tcp_bufwriter(&tcp_stream);

    log::info!("Connecting to: {ip}:{port}");
    if let Some(msg) = cfg.message() {
        let res = buf_tcp_stream.write_all(msg.as_bytes());
        log::debug!("Wrote message: {msg}");
        log::debug!("TCP write result: {res:?}");
    }

    // On-stack dynamic dispatch
    let (mut stdin_read, mut file_read, mut mmap_read);
    let bufreader: &mut dyn io::Read = match cfg.file() {
        Some(p) if cfg.use_mmap() => {
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

    let compression_mode = cfg.compression().unwrap_or_default();
    log::debug!("Compression mode: {compression_mode}");
    let transferred_bytes = match cfg.compression().unwrap_or_default() {
        config::Compression::Lz4 => {
            let mut lz4_writer = lz4_flex::frame::FrameEncoder::new(&mut buf_tcp_stream);
            let total_read = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut lz4_writer, bufreader)?;
            lz4_writer.finish()?;
            total_read
        }
        config::Compression::Gzip => {
            let mut encoder = GzEncoder::new(bufreader, Compression::fast());
            incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut encoder)?
        }
        config::Compression::Bzip2 => todo!(),
        config::Compression::Xz => todo!(),
        config::Compression::None => {
            incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, bufreader)?
        }
    };
    log::info!(
        "Sent {} [{transferred_bytes} B]",
        format_data_size(transferred_bytes)
    );

    Ok(())
}

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
