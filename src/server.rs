use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdoutLock, Write},
    net::{IpAddr, TcpListener, TcpStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    config::{
        compression::CompressionVariant,
        transfer::{command::ServerCommand, listen::ListenArgs},
        Config,
    },
    util::{
        bind_listen_to_free_port_in_range, create_file_with_len, format_data_size, incremental_rw,
        read_server_cmd, server_handshake,
    },
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};
use anyhow::{bail, Result};

pub fn listen(_cfg: &Config, listen_args: &ListenArgs) -> Result<()> {
    let ListenArgs {
        ip,
        port,
        output: _,
        decompression: _,
        output_dir: _,
    } = listen_args;

    let stop_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let port = port.unwrap();
    let ip: IpAddr = ip.parse()?;

    let initial_listener = TcpListener::bind((ip, port))?;

    let mut handles = vec![];

    match initial_listener.accept() {
        Ok((mut socket, addr)) => {
            log::debug!("Client accepted at: {addr:?}");
            server_handshake(&mut socket)?;
            let mut cmd_buf: [u8; 256] = [0; 256];
            loop {
                if let Some(cmd) = read_server_cmd(&mut socket, &mut cmd_buf)? {
                    log::trace!("Received command: {cmd:?}");

                    if matches!(cmd, ServerCommand::GetFreePort(_)) {
                        let (start_port_range, end_port_range) = match cmd {
                            ServerCommand::GetFreePort((start_port_range, end_port_range)) => {
                                (start_port_range, end_port_range)
                            }
                            _ => unreachable!(),
                        };
                        let start = start_port_range.unwrap_or(49152);
                        let end = end_port_range.unwrap_or(61000);
                        let thread_listener: TcpListener = match bind_listen_to_free_port_in_range(
                            &listen_args.ip,
                            start,
                            end,
                        ) {
                            Some(listener) => listener,
                            None => {
                                log::error!("Unable to find free port in range {start}-{end}, attempting to bind to any free port");
                                TcpListener::bind((listen_args.ip.as_str(), 0))?
                            }
                        };
                        let free_port = thread_listener
                            .local_addr()
                            .expect("Unable to get local address for TCP listener")
                            .port();
                        let free_port_be_bytes = free_port.to_be_bytes();
                        debug_assert_eq!(free_port_be_bytes.len(), 2);
                        socket.write_all(&free_port_be_bytes)?;
                        socket.flush()?;
                        let s = std::thread::Builder::new().name(port.to_string());
                        let h: JoinHandle<anyhow::Result<()>> = s
                            .spawn({
                                let cfg = listen_args.clone();
                                let local_stop_flag = Arc::clone(&stop_flag);
                                move || {

                                    thread_listener.set_nonblocking(true)?;

                                    for client in thread_listener.incoming() {
                                        match client {
                                            Ok(mut socket) => {
                                                log::trace!("Got client");
                                                server_handshake(&mut socket)?;
                                                let mut cmd_buf: [u8; 256] = [0; 256];

                                                loop {
                                                    log::info!("Ready to receive command");
                                                    if let Some(cmd) =
                                                    read_server_cmd(&mut socket, &mut cmd_buf)?
                                                    {
                                                        log::trace!("Received command: {cmd:?}");
                                                        match cmd {
                                                            ServerCommand::Prealloc(fsize, fname) => {
                                                                log::debug!(
                                                                "Preallocating file of size {} [{fsize} B]",
                                                                format_data_size(fsize)
                                                            );
                                                                if let Some(out_dir) = cfg.output_dir.as_deref() {
                                                                    if !out_dir.is_dir() && out_dir.exists() {
                                                                        bail!("Output directory path {out_dir:?} is invalid - has to point at a directory or non-existent path")
                                                                    }
                                                                    if !out_dir.exists() {
                                                                        fs::create_dir(out_dir)?;
                                                                    }
                                                                    let out_file = out_dir.join(fname);
                                                                    log::trace!("Preallocating for path: {out_dir:?}");
                                                                    create_file_with_len(&out_file, fsize)?;
                                                                } else if let Some(out_file) = cfg.output.as_deref() {
                                                                    create_file_with_len(out_file, fsize)?;
                                                                }
                                                            }
                                                            ServerCommand::ReceiveData(
                                                                _f_count,
                                                                fname,
                                                                decompr,
                                                            ) => {
                                                                log::debug!(
                                                                    "Received file list: {fname:?}"
                                                                );
                                                                handle_receive_data(
                                                                    &cfg,
                                                                    &mut socket,
                                                                    fname,
                                                                    decompr,
                                                                )?;
                                                            }
                                                            ServerCommand::GetFreePort(_) => todo!(),
                                                        }
                                                    } else {
                                                        log::info!("[thread] Client disconnected...");
                                                        break;
                                                    }
                                                }
                                            },

                                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                                log::trace!("Would block");
                                                if cfg!(linux) {
                                                    log::trace!("Would block - yielding thread");
                                                    std::thread::park_timeout(Duration::from_millis(100));
                                                }
                                            }

                                            Err(e) => bail!(e),
                                        };
                                        if local_stop_flag.load(Ordering::Relaxed) {
                                            break;
                                        }
                                    }

                                    Ok(())
                                }
                            })
                            .expect("Failed spawning thread");
                        handles.push(h);
                    }
                } else {
                    log::info!("Main Client disconnected...");
                    break;
                }
            }
        }
        Err(e) => bail!(e),
    }

    stop_flag.store(true, Ordering::Relaxed);
    for h in handles {
        //
        match h.join().expect("Failed to join thread") {
            Ok(_) => (),
            Err(e) => log::error!("Thread joined with error: {e}"),
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
