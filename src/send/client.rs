use std::{
    fs::File,
    io::{self, Write},
    net::{IpAddr, TcpStream},
};

use crate::{
    config::{self, transfer::ContentTransferArgs},
    mmap_reader::MemoryMappedReader,
    send::util::{file_with_bufreader, stdin_bufreader, tcp_bufwriter},
    util::{format_data_size, incremental_rw},
    TCP_STREAM_BUFSIZE,
};
use anyhow::Result;

pub fn run_client(
    ip: IpAddr,
    port: u16,
    message: Option<&str>,
    use_mmap: bool,
    content_transfer_args: &ContentTransferArgs,
) -> Result<()> {
    let socket_addr = (ip, port);
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    if content_transfer_args.prealloc() {
        let file_size = File::open(content_transfer_args.file().unwrap())?
            .metadata()?
            .len();
        log::debug!(
            "Requesting preallocation of file of size {} [{file_size} B]",
            format_data_size(file_size)
        );
        tcp_stream.write_all(&file_size.to_be_bytes())?;
    }
    let mut buf_tcp_stream = tcp_bufwriter(&tcp_stream);

    log::info!("Connecting to: {ip}:{port}");
    if let Some(msg) = message {
        let res = buf_tcp_stream.write_all(msg.as_bytes());
        log::debug!("Wrote message: {msg}");
        log::debug!("TCP write result: {res:?}");
    }

    // On-stack dynamic dispatch
    let (mut stdin_read, mut file_read, mut mmap_read);
    let bufreader: &mut dyn io::Read = match content_transfer_args.file() {
        Some(p) if use_mmap => {
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

    if let Some(compression) = content_transfer_args.compression() {
        log::debug!("Compression mode: {compression}");
    };
    let transferred_bytes = match content_transfer_args.compression() {
        Some(compression) => match compression {
            config::compression::Compression::Lz4 => {
                let mut lz4_writer = lz4_flex::frame::FrameEncoder::new(&mut buf_tcp_stream);
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut lz4_writer, bufreader)?
            }
            config::compression::Compression::Gzip => {
                let mut encoder =
                    flate2::read::GzEncoder::new(bufreader, flate2::Compression::fast());
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Bzip2 => {
                let mut encoder =
                    bzip2::read::BzEncoder::new(bufreader, bzip2::Compression::best());
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Xz => {
                let mut compressor = xz2::read::XzEncoder::new(bufreader, 9);
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut compressor)?
            }
        },
        None => incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, bufreader)?,
    };
    log::info!(
        "Sent {} [{transferred_bytes} B]",
        format_data_size(transferred_bytes)
    );

    Ok(())
}
