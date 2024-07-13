use std::{
    fs,
    io::{self},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::bail;

use crate::{
    config::transfer::{command::ServerCommand, listen::ListenArgs},
    server::util::handle_receive_data,
    util::{create_file_with_len, read_server_cmd, server_handshake},
};

pub fn run_child(
    listener: &TcpListener,
    cfg: &ListenArgs,
    stop_flag: &Arc<AtomicBool>,
) -> anyhow::Result<()> {
    for client in listener.incoming() {
        match client {
            Ok(mut socket) => {
                handle_child_socket(cfg, &mut socket)?;
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

pub fn handle_child_socket(cfg: &ListenArgs, socket: &mut TcpStream) -> anyhow::Result<()> {
    socket
        .set_nonblocking(false)
        .expect("Failed putting socket into blocking state");
    tracing::trace!("{socket:?}");
    tracing::trace!("Got client at {:?}", socket.local_addr());
    server_handshake(socket)?;
    let mut cmd_buf: [u8; 256] = [0; 256];

    loop {
        log::info!("Ready to receive command");
        if let Some(cmd) = read_server_cmd(socket, &mut cmd_buf)? {
            log::trace!("Received command: {cmd:?}");
            handle_child_cmd(cmd, cfg, socket)?;
        } else {
            log::info!("[thread] Client disconnected...");
            break;
        }
    }
    Ok(())
}

pub fn handle_child_cmd(
    cmd: ServerCommand,
    cfg: &ListenArgs,
    socket: &mut TcpStream,
) -> anyhow::Result<()> {
    match cmd {
        ServerCommand::Prealloc(fsize, fname) => {
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
            handle_receive_data(cfg, socket, fname, decompr)?;
        }
        // TODO: Constrict these to only the main thread.
        ServerCommand::GetFreePort(_) => todo!(),
        ServerCommand::EndOfTransfer => {
            unreachable!("Child thread received end of transfer command")
        }
        ServerCommand::IsDestinationValid(_, _) => todo!(),
    }
    Ok(())
}
