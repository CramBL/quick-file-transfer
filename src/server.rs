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
        transfer::{command::ServerCommand, listen::ListenArgs},
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
                        let child_thread_handle = spawn_server_thread_on_new_port(
                            &mut socket,
                            listen_args,
                            &Arc::clone(&stop_flag),
                            &cmd,
                        )?;
                        handles.push(child_thread_handle);
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
    join_all_threads(handles);

    Ok(())
}

fn join_all_threads(handles: Vec<JoinHandle<Result<(), anyhow::Error>>>) {
    for h in handles {
        match h.join().expect("Failed to join thread") {
            Ok(_) => (),
            Err(e) => log::error!("Thread joined with error: {e}"),
        }
    }
}

fn handle_cmd(cmd: ServerCommand, cfg: &ListenArgs, socket: &mut TcpStream) -> anyhow::Result<()> {
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
        ServerCommand::ReceiveData(_f_count, fname, decompr) => {
            log::debug!("Received file list: {fname:?}");
            util::handle_receive_data(cfg, socket, fname, decompr)?;
        }
        ServerCommand::GetFreePort(_) => todo!(),
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
                log::trace!("Would block");
                if cfg!(linux) {
                    log::trace!("Would block - yielding thread");
                    std::thread::park_timeout(Duration::from_millis(100));
                }
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
