use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdoutLock, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};

use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;

use crate::{
    config::{
        compression::CompressionVariant,
        transfer::{
            command::{ServerCommand, ServerResult},
            listen::ListenArgs,
        },
    },
    server::child::run_child,
    util::{bind_listen_to_free_port_in_range, format_data_size, incremental_rw},
    BUFFERED_RW_BUFSIZE, TCP_STREAM_BUFSIZE,
};

pub fn file_with_bufwriter(path: &Path) -> anyhow::Result<BufWriter<File>> {
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

pub fn handle_receive_data(
    listen_args: &ListenArgs,
    tcp_socket: &mut TcpStream,
    fname: String,
    decompression: Option<CompressionVariant>,
    root_dest: Option<&Path>,
) -> anyhow::Result<u64> {
    let mut bufwriter = match (
        listen_args.output.as_deref(),
        listen_args.output_dir.as_deref(),
        root_dest,
    ) {
        (_, _, Some(root_dest)) => {
            if root_dest.is_file() {
                tracing::info!("Initiation bufwriter targeting {root_dest:?}");
                file_with_bufwriter(root_dest)?
            } else {
                let full_path = root_dest.join(fname);
                tracing::info!("Initiation bufwriter targeting {full_path:?}");
                file_with_bufwriter(&full_path)?
            }
        }
        (None, Some(d), _) => {
            if !d.is_dir() && d.exists() {
                anyhow::bail!("Output directory path {d:?} is invalid - has to point at a directory or non-existent path")
            }
            if !d.exists() {
                fs::create_dir(d)?;
            }
            let new_fpath = d.join(fname);
            file_with_bufwriter(&new_fpath)?
        }
        (Some(f), None, _) => file_with_bufwriter(f)?,
        (None, None, _) => {
            unreachable!()
        }
        (Some(_), Some(_), _) => {
            unreachable!("Specifying both an output name and an output directory is invalid")
        }
    };

    let mut buf_tcp_reader = BufReader::with_capacity(BUFFERED_RW_BUFSIZE, tcp_socket);

    let len = match decompression {
        Some(compr) => match compr {
            CompressionVariant::Bzip2 => {
                let mut tcp_decoder = bzip2::read::BzDecoder::new(buf_tcp_reader);
                incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut bufwriter, &mut tcp_decoder)?
            }
            CompressionVariant::Gzip => {
                let mut tcp_decoder = GzDecoder::new(buf_tcp_reader);
                incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut bufwriter, &mut tcp_decoder)?
            }
            CompressionVariant::Lz4 => {
                let mut tcp_decoder = FrameDecoder::new(buf_tcp_reader);
                incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut bufwriter, &mut tcp_decoder)?
            }
            CompressionVariant::Xz => {
                let mut tcp_decoder = xz2::read::XzDecoder::new(buf_tcp_reader);
                incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut bufwriter, &mut tcp_decoder)?
            }
        },
        None => incremental_rw::<TCP_STREAM_BUFSIZE, _, _>(&mut bufwriter, &mut buf_tcp_reader)?,
    };
    if len < 1023 {
        log::info!("Received: {len} B");
    } else {
        log::info!("Received: {} [{len} B]", format_data_size(len));
    }

    Ok(len)
}

/// Send a [ServerResult] to the client
pub fn send_result(stream: &mut TcpStream, result: &ServerResult) -> anyhow::Result<()> {
    tracing::trace!("Sending result: {result:?}");
    let result_bytes = bincode::serialize(result)?;
    debug_assert!(result_bytes.len() <= u8::MAX as usize);
    let size = result_bytes.len() as u16;
    let header = size.to_be_bytes();

    // Send the header followed by the command
    stream.write_all(&header)?;
    stream.write_all(&result_bytes)?;
    Ok(())
}

pub fn join_all_threads(handles: Vec<JoinHandle<anyhow::Result<()>>>) -> Result<(), String> {
    let mut errors = String::new();
    for h in handles {
        let mut h_name = h.thread().name().unwrap_or_default().to_owned();
        match h.join().map_err(|e| format!("{e:?}")) {
            Ok(_) => (),
            Err(e) => {
                tracing::error!("Thread {h_name} joined with error: {e}");
                h_name.push_str(" failed: ");
                if !errors.is_empty() {
                    errors.push('\n');
                }
                errors.extend(h_name.drain(..));
                errors.push_str(&e);
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        tracing::warn!("{errors}");
        Err(errors)
    }
}

pub fn spawn_child_on_new_port(
    socket: &mut TcpStream,
    cfg: &ListenArgs,
    stop_flag: &Arc<AtomicBool>,
    server_cmd_get_free_port: &ServerCommand,
    root_dest: Option<PathBuf>,
) -> anyhow::Result<JoinHandle<anyhow::Result<()>>> {
    let (start_port_range, end_port_range) = match server_cmd_get_free_port {
        ServerCommand::GetFreePort((start_port_range, end_port_range)) => {
            (start_port_range, end_port_range)
        }
        _ => unreachable!(),
    };
    let start = start_port_range.unwrap_or(49152);
    let end = end_port_range.unwrap_or(61000);
    let thread_listener: TcpListener = match bind_listen_to_free_port_in_range(&cfg.ip, start, end)
    {
        Some(listener) => listener,
        None => {
            log::error!("Unable to find free port in range {start}-{end}, attempting to bind to any free port");
            TcpListener::bind((cfg.ip.as_str(), 0))?
        }
    };
    let free_port = thread_listener
        .local_addr()
        .expect("Unable to get local address for TCP listener")
        .port();
    tracing::trace!("Bound to free port: {free_port}");

    let free_port_be_bytes = free_port.to_be_bytes();
    debug_assert_eq!(free_port_be_bytes.len(), 2);
    socket.write_all(&free_port_be_bytes)?;
    socket.flush()?;
    let thread_builder = std::thread::Builder::new().name(format!("ThreadOn#{free_port}"));
    let handle: JoinHandle<anyhow::Result<()>> = thread_builder
        .spawn({
            let cfg = cfg.clone();
            let local_stop_flag = Arc::clone(stop_flag);
            move || {
                thread_listener.set_nonblocking(true)?;
                run_child(
                    &thread_listener,
                    &cfg,
                    &local_stop_flag,
                    root_dest.as_deref(),
                )
            }
        })
        .expect("Failed spawning thread");
    Ok(handle)
}
