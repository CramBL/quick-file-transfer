use crate::{
    config::{
        transfer::{
            command::{ServerCommand, ServerResult},
            listen::ListenArgs,
        },
        Config,
    },
    util::{read_server_cmd, server_handshake},
};
use anyhow::{bail, Result};
use std::{
    net::{IpAddr, TcpListener},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

mod path;
use path::validate_remote_path;

pub mod util;
use util::{join_all_threads, send_result, spawn_child_on_new_port};

pub mod child;

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
    run_server(&initial_listener, listen_args, &stop_flag)
}

fn run_server(
    initial_listener: &TcpListener,
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
                    match cmd {
                        ServerCommand::GetFreePort(_) => {
                            let child_thread_handle = spawn_child_on_new_port(
                                &mut socket,
                                args,
                                &Arc::clone(stop_flag),
                                &cmd,
                            )?;
                            thread_handles.push(child_thread_handle);
                        }
                        ServerCommand::EndOfTransfer => {
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
                        ServerCommand::Prealloc(_, _) => todo!(),
                        ServerCommand::ReceiveData(_, _, _) => todo!(),
                        ServerCommand::IsDestinationValid(mode, dest) => {
                            let dest = PathBuf::from(dest);
                            tracing::info!("Checking validity of remote path: {dest:?}");
                            match validate_remote_path(&mode, &dest) {
                                Ok(_) => {
                                    send_result(&mut socket, &ServerResult::Ok)?;
                                }
                                Err(e) => {
                                    tracing::error!("Invalid remote path: {e}");
                                    send_result(
                                        &mut socket,
                                        &ServerResult::Err(e.to_string().into()),
                                    )?;
                                }
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
