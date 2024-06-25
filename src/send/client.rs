use std::{
    fs::File,
    io::{self, Write},
    net::{IpAddr, TcpStream},
    path::Path,
};

use crate::{
    config::{
        self,
        compression::{Bzip2Args, Compression, GzipArgs, XzArgs},
        transfer::ContentTransferArgs,
    },
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
    compression: Option<Compression>,
) -> Result<()> {
    let socket_addr = (ip, port);
    let mut tcp_stream = TcpStream::connect(socket_addr)?;
    if content_transfer_args.prealloc() {
        let file_size = File::open(content_transfer_args.file().unwrap())?
            .metadata()?
            .len();
        log::debug!(
            "Requesting preallocation of file of size {} [{file_size} B]",
            format_data_size(file_size as u64)
        );
        tcp_stream.write_all(&file_size.to_be_bytes())?;
    }

    let transferred_len = transfer_data(
        (ip, port),
        &tcp_stream,
        compression,
        content_transfer_args.file(),
        message,
        use_mmap,
    )?;

    log::info!(
        "Sent {} [{transferred_len} B]",
        format_data_size(transferred_len)
    );

    Ok(())
}

fn transfer_data(
    (ip, port): (IpAddr, u16),
    tcp_stream: &TcpStream,
    compression: Option<Compression>,
    file: Option<&Path>,
    message: Option<&str>,
    use_mmap: bool,
) -> Result<u64> {
    let mut buf_tcp_stream = tcp_bufwriter(&tcp_stream);

    log::info!("Connecting to: {ip}:{port}");
    if let Some(msg) = message {
        let res = buf_tcp_stream.write_all(msg.as_bytes());
        log::debug!("Wrote message: {msg}");
        log::debug!("TCP write result: {res:?}");
    }

    // On-stack dynamic dispatch
    let (mut stdin_read, mut file_read, mut mmap_read);
    let bufreader: &mut dyn io::Read = match file {
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

    if let Some(compression) = compression {
        log::debug!("Compression mode: {compression}");
    };

    let transferred_bytes = match compression {
        Some(compression) => match compression {
            config::compression::Compression::Bzip2(Bzip2Args { compression_level }) => {
                let mut encoder = bzip2::read::BzEncoder::new(
                    bufreader,
                    bzip2::Compression::new(compression_level.into()),
                );
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Lz4 => {
                let mut lz4_writer = lz4_flex::frame::FrameEncoder::new(&mut buf_tcp_stream);
                let len = incremental_rw::<TCP_STREAM_BUFSIZE>(&mut lz4_writer, bufreader)?;
                lz4_writer.flush()?; // Needed to ensure the entire content is written
                len
            }
            config::compression::Compression::Gzip(GzipArgs { compression_level }) => {
                let mut encoder = flate2::read::GzEncoder::new(
                    bufreader,
                    flate2::Compression::new(compression_level.into()),
                );
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut encoder)?
            }
            config::compression::Compression::Xz(XzArgs { compression_level }) => {
                let mut compressor = xz2::read::XzEncoder::new(bufreader, compression_level.into());
                incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, &mut compressor)?
            }
        },
        None => incremental_rw::<TCP_STREAM_BUFSIZE>(&mut buf_tcp_stream, bufreader)?,
    };
    Ok(transferred_bytes)
}
