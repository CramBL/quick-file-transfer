use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdoutLock},
    net::{TcpListener, TcpStream},
    path::Path,
};

use crate::{
    config::{
        compression::CompressionVariant,
        transfer::{command::ServerCommand, listen::ListenArgs},
        Config,
    },
    util::{
        create_file_with_len, format_data_size, incremental_rw, read_server_cmd, server_handshake,
    },
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::{bail, Result};

pub fn listen(_cfg: &Config, listen_args: &ListenArgs) -> Result<()> {
    let ListenArgs {
        ip,
        port,
        output: _,
        decompression,
        output_dir: _,
    } = listen_args;

    let port = port.unwrap();
    let listener = TcpListener::bind(format!("{ip}:{port}"))?;
    log::info!(
        "Listening on: {ip}:{port} for a {describe_contents}",
        describe_contents = match decompression {
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

    let mut expect_data_recv_cnt = 1;
    while expect_data_recv_cnt > 0 {
        match listener.accept() {
            Ok((mut socket, addr)) => {
                expect_data_recv_cnt -= 1;
                log::debug!("Client accepted at: {addr:?}");
                server_handshake(&mut socket)?;

                let mut cmd_buf: [u8; 256] = [0; 256];
                // Main command receive event loop
                loop {
                    if let Some(cmd) = read_server_cmd(&mut socket, &mut cmd_buf)? {
                        log::trace!("Received command: {cmd:?}");
                        let expect_more_data = command_handler(&mut socket, cmd, listen_args)?;
                        log::trace!("Expecting {expect_more_data} more data transfers");
                        expect_data_recv_cnt = expect_more_data;
                    } else {
                        log::info!("Client disconnected...");
                        break;
                    }
                }
            }
            Err(e) => bail!(e),
        }
    }
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
                let fpath_str = path.display().to_string();
                let file_permissions: Option<fs::Permissions> = if file_exists {
                    if let Ok(md) = path.metadata() {
                        Some(md.permissions())
                    } else {
                        log::error!("Failed to retrieve permissions for {fpath_str}");
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
                    context_str.push_str(&format!("\n\tFile {fpath_str} exists on disk"));
                } else {
                    context_str.push_str(&format!("\n\tFile {fpath_str} does not exist"));
                }
                if let Some(fpermission) = file_permissions {
                    context_str.push_str(&format!(" - with permissions: {fpermission:?}"));
                }
                if let Some(parent_permissions) = parent_permissions {
                    context_str.push_str(&format!(
                        "\n\tParent directory {:?} - permissions: {parent_permissions:?}",
                        parent.unwrap(),
                    ));
                }
                log::debug!("Additional context for {fpath_str}:{context_str}");
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

#[allow(clippy::needless_pass_by_value)]
fn command_handler(
    tcp_socket: &mut TcpStream,
    cmd: ServerCommand,
    listen_args: &ListenArgs,
) -> anyhow::Result<u32> {
    match cmd {
        ServerCommand::ReceiveData(f_count, fname, decompr) => {
            log::debug!("Received file list: {fname:?}");
            handle_receive_data(listen_args, tcp_socket, fname, decompr)?;
            return Ok(f_count);
        }
        ServerCommand::GetFreePort => todo!(),
        ServerCommand::Prealloc(fsize) => {
            log::debug!(
                "Preallocating file of size {} [{fsize} B]",
                format_data_size(fsize)
            );
            create_file_with_len(listen_args.output.as_deref().unwrap(), fsize)?;
        }
    }

    Ok(0)
}

fn handle_receive_data(
    listen_args: &ListenArgs,
    tcp_socket: &mut TcpStream,
    fname: String,
    decompression: Option<CompressionVariant>,
) -> anyhow::Result<u64> {
    // On-stack dynamic dispatch
    let (mut stdout_write, mut file_write);

    let bufwriter: &mut dyn io::Write = match (
        listen_args.output.as_deref(),
        listen_args.output_dir.as_deref(),
    ) {
        (None, Some(d)) => {
            if !d.is_dir() && d.exists() {
                bail!("Output directory path {d:?} is invalid - has to point at a directory or non-existent path")
            }
            if !d.exists() {
                fs::create_dir(d)?;
            }
            let new_fpath = d.join(fname);
            file_write = file_with_bufwriter(&new_fpath)?;
            &mut file_write
        }
        (Some(f), None) => {
            file_write = file_with_bufwriter(f)?;
            &mut file_write
        }
        (None, None) => {
            stdout_write = stdout_bufwriter();
            &mut stdout_write
        }
        (Some(_), Some(_)) => {
            unreachable!("Specifying both an output name and an output directory is invalid")
        }
    };

    let mut buf_tcp_reader = BufReader::with_capacity(BUFFERED_RW_BUFSIZE, tcp_socket);

    let len = match decompression {
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
    if len < 1023 {
        log::info!("Received: {len} B");
    } else {
        log::info!("Received: {} [{len} B]", format_data_size(len));
    }

    Ok(len)
}
