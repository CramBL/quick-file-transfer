use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, StdoutLock},
    net::TcpListener,
    path::Path,
};

use crate::{
    config::{compression::CompressionVariant, transfer::listen::ListenArgs, Config},
    util::{create_file_with_len, format_data_size, incremental_rw},
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::Result;

pub fn listen(_cfg: &Config, listen_args: &ListenArgs) -> Result<()> {
    let ListenArgs {
        ip,
        port,
        prealloc,
        output: output_file,
        compression,
    } = listen_args;
    let listener = TcpListener::bind(format!("{ip}:{port}"))?;

    log::info!(
        "Listening on: {ip}:{port} for a {}",
        match compression {
            Some(c) => {
                match c {
                    CompressionVariant::Bzip2 => "bzip2 compressed file",
                    CompressionVariant::Gzip => "gzip compressed file",
                    CompressionVariant::Lz4 => "lz4 compressed file",
                    CompressionVariant::Xz => "xz compressed file",
                }
            }
            None => "raw file",
        }
    );
    // On-stack dynamic dispatch
    let (mut stdout_write, mut file_write);
    let bufwriter: &mut dyn io::Write = match output_file {
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
            log::debug!("Client accepted at: {addr:?}");
            if *prealloc {
                let mut size_buffer = [0u8; 8];
                socket.read_exact(&mut size_buffer)?;
                let file_size = u64::from_be_bytes(size_buffer);
                log::debug!(
                    "Preallocating file of size {} [{file_size} B]",
                    format_data_size(file_size)
                );
                create_file_with_len(output_file.as_deref().unwrap(), file_size)?;
            }
            let mut buf_tcp_reader = BufReader::with_capacity(BUFFERED_RW_BUFSIZE, socket);

            let len = match compression {
                Some(compr) => match compr {
                    CompressionVariant::Bzip2 => {
                        let mut tcp_decoder = bzip2::read::BzDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                    CompressionVariant::Gzip => {
                        let mut tcp_decoder = GzDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                    CompressionVariant::Lz4 => {
                        let mut tcp_decoder = FrameDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                    CompressionVariant::Xz => {
                        let mut tcp_decoder = xz2::read::XzDecoder::new(buf_tcp_reader);
                        incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut tcp_decoder)?
                    }
                },
                None => incremental_rw::<TCP_STREAM_BUFSIZE>(bufwriter, &mut buf_tcp_reader)?,
            };
            log::info!("Received: {} [{len} B]", format_data_size(len));
        }
        Err(e) => println!("Failed accepting connection to client: {e:?}"),
    }

    log::debug!("Server exiting...");
    Ok(())
}

pub fn file_with_bufwriter(path: &Path) -> Result<BufWriter<File>> {
    let f = match fs::File::create(path) {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == io::ErrorKind::PermissionDenied {
                log::error!("{e}");
                log::info!("Attempting to retrieve additional debug information...");
                let file_exists = path.exists();
                let file_permissions: Option<fs::Permissions> = if file_exists {
                    if let Ok(md) = path.metadata() {
                        Some(md.permissions())
                    } else {
                        log::error!("Failed to retrieve permissions for {}", path.display());
                        None
                    }
                } else {
                    None
                };

                let parent = path.parent();
                let parent_permissions: Option<fs::Permissions> =
                    parent.and_then(|p| p.metadata().ok().map(|md| md.permissions()));
                let mut context_str = String::new();
                if file_exists {
                    context_str.push_str("\n\tFile exists on disk");
                } else {
                    context_str.push_str("\n\tFile does not exist");
                }
                if let Some(fpermission) = file_permissions {
                    context_str.push_str(&format!(" - with permissions: {fpermission:?}"));
                }
                if let Some(parent_permissions) = parent_permissions {
                    context_str.push_str(&format!(
                        "Parent directory {parent:?} - permissions: {parent_permissions:?}"
                    ));
                }
                log::debug!("Additional context for {}:{context_str}", path.display(),);
            };
            return Err(e.into());
        }
    };
    let writer = BufWriter::with_capacity(BUFFERED_RW_BUFSIZE, f);
    Ok(writer)
}

pub fn stdout_bufwriter() -> BufWriter<StdoutLock<'static>> {
    let stdout = io::stdout().lock();
    BufWriter::with_capacity(BUFFERED_RW_BUFSIZE, stdout)
}
