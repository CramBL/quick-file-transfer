use std::{
    fs::{self},
    io::{self, Write},
    net::{IpAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    config::{
        transfer::{
            command::{ServerCommand, ServerResult},
            listen::ListenArgs,
        },
        Config,
    },
    util::{
        bind_listen_to_free_port_in_range, create_file_with_len, format_data_size, read_server_cmd,
        server_handshake,
    },
};
use anyhow::{bail, Result};

pub mod util;

pub fn listen(_cfg: &Config, listen_args: &ListenArgs) -> Result<()> {
    let ListenArgs {
        ip,
        port,
        output: _,
        decompression: _,
        output_dir: _,
        remote: _,
    } = listen_args;

    let stop_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let ip: IpAddr = ip.parse()?;
    let initial_listener = TcpListener::bind((ip, *port))?;
    run_server(initial_listener, listen_args, &stop_flag)
}

fn run_server(
    initial_listener: TcpListener,
    args: &ListenArgs,
    stop_flag: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut thread_handles = vec![];
    match initial_listener.accept() {
        Ok((mut socket, addr)) => {
            tracing::info!("Client accepted at: {addr:?}");
            server_handshake(&mut socket)?;
            let mut cmd_buf: [u8; 256] = [0; 256];
            loop {
                if let Some(cmd) = read_server_cmd(&mut socket, &mut cmd_buf)? {
                    tracing::trace!("Received command: {cmd:?}");
                    if matches!(cmd, ServerCommand::GetFreePort(_)) {
                        let child_thread_handle = spawn_server_thread_on_new_port(
                            &mut socket,
                            args,
                            &Arc::clone(&stop_flag),
                            &cmd,
                        )?;
                        thread_handles.push(child_thread_handle);
                    } else if matches!(cmd, ServerCommand::EndOfTransfer) {
                        tracing::trace!("Received command: {cmd:?}, stopping all threads...");
                        stop_flag.store(true, Ordering::Relaxed);
                        match join_all_threads(thread_handles) {
                            Ok(_) => {
                                send_result(&mut socket, &ServerResult::Ok)?;
                                return Ok(());
                            }
                            Err(th_errs) => {
                                let err_res = ServerResult::err(th_errs.clone());
                                send_result(&mut socket, &err_res)?;
                                bail!(th_errs);
                            }
                        }
                    }
                } else {
                    tracing::debug!("Main Client disconnected...");
                    break;
                }
            }
        }
        Err(e) => bail!(e),
    }
    Ok(())
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

fn join_all_threads(handles: Vec<JoinHandle<anyhow::Result<()>>>) -> Result<(), String> {
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

fn handle_cmd(cmd: ServerCommand, cfg: &ListenArgs, socket: &mut TcpStream) -> anyhow::Result<()> {
    match cmd {
        ServerCommand::Prealloc(fsize, fname) => {
            tracing::debug!(
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
        ServerCommand::ReceiveData(_f_count, fname, decompr) => {
            log::debug!("Received file list: {fname:?}");
            util::handle_receive_data(cfg, socket, fname, decompr)?;
        }
        // TODO: Constrict these to only the main thread.
        ServerCommand::GetFreePort(_) => todo!(),
        ServerCommand::EndOfTransfer => {
            unreachable!("Child thread received end of transfer command")
        }
    }
    Ok(())
}

fn handle_client_socket(cfg: &ListenArgs, socket: &mut TcpStream) -> anyhow::Result<()> {
    socket
        .set_nonblocking(false)
        .expect("Failed putting socket into blocking state");
    log::trace!("{socket:?}");
    log::trace!("Got client at {:?}", socket.local_addr());
    server_handshake(socket)?;
    let mut cmd_buf: [u8; 256] = [0; 256];

    loop {
        log::info!("Ready to receive command");
        if let Some(cmd) = read_server_cmd(socket, &mut cmd_buf)? {
            log::trace!("Received command: {cmd:?}");
            handle_cmd(cmd, cfg, socket)?;
        } else {
            log::info!("[thread] Client disconnected...");
            break;
        }
    }
    Ok(())
}

fn run_server_thread(
    listener: &TcpListener,
    cfg: &ListenArgs,
    stop_flag: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    for client in listener.incoming() {
        match client {
            Ok(mut socket) => {
                handle_client_socket(cfg, &mut socket)?;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                log::trace!("Would block - yielding thread");
                std::thread::park_timeout(Duration::from_millis(10));
            }
            Err(e) => {
                log::error!("{e}");
                bail!(e)
            }
        };
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
    }
    Ok(())
}

fn spawn_server_thread_on_new_port(
    socket: &mut TcpStream,
    cfg: &ListenArgs,
    stop_flag: &Arc<AtomicBool>,
    server_cmd_get_free_port: &ServerCommand,
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
                run_server_thread(&thread_listener, &cfg, &local_stop_flag)
            }
        })
        .expect("Failed spawning thread");
    Ok(handle)
}
