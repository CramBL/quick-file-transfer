use crate::{
    config::{
        compression::Compression,
        transfer::{send::ssh::SendSshArgs, util::TcpConnectMode},
        Config,
    },
    util::verbosity_to_args,
};
use anyhow::{bail, Result};
use std::{
    ffi::OsStr,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    thread::ScopedJoinHandle,
    time::Duration,
};

#[cfg(feature = "mdns")]
mod mdns_util;
pub mod private_key;
mod remote_cmd;
pub mod remote_find_free_port;
pub mod remote_info;
pub(crate) mod util;

pub const ENV_REMOTE_PASSWORD: &str = "QFT_REMOTE_PASSWORD";
pub const ENV_SSH_KEY_DIR: &str = "QFT_SSH_KEY_DIR";
pub const ENV_SSH_PRIVATE_KEY: &str = "QFT_SSH_PRIVATE_KEY";

pub const ENV_REMOTE_USER: &str = "QFT_REMOTE_USER";

pub fn handle_send_ssh(
    cfg: &Config,
    args: &SendSshArgs,
    input_files: &[PathBuf],
    prealloc: bool,
    use_mmap: bool,
    tcp_conect_mode: TcpConnectMode,
) -> Result<()> {
    let remote_info = remote_info::RemoteInfo::from_args(args);
    let SendSshArgs {
        user: _,
        #[cfg(feature = "mdns")]
            hostname: _,
        #[cfg(feature = "mdns")]
            ip_version: _,
        mdns_resolve_timeout_ms: _,
        ssh_port: _,
        compression,
        ip: _,
        target: _,
        destination: _,
        tcp_port,
        ssh_private_key_path,
        ssh_key_dir,
        start_port,
        end_port,
        ssh_timeout_ms,
        tcp_delay_ms,
    } = args;

    let ssh_private_key = private_key::get_ssh_private_key_path(
        ssh_private_key_path.as_deref(),
        ssh_key_dir.as_deref(),
    )?
    .into_os_string();
    log::info!("{ssh_private_key:?}");

    run_ssh(
        cfg,
        (
            remote_info.user,
            util::get_remote_password_from_env()
                .as_deref()
                .unwrap_or("root"),
        ),
        &ssh_private_key,
        &remote_info.resolved_ip,
        remote_info.destination.as_ref(),
        remote_info.ssh_port,
        *tcp_port,
        use_mmap,
        input_files,
        prealloc,
        compression,
        *start_port,
        *end_port,
        *ssh_timeout_ms,
        *tcp_delay_ms,
        tcp_conect_mode,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_ssh(
    cfg: &Config,
    (username, password): (&str, &str),
    priv_key_path: &OsStr,
    remote_ip: &str,
    remote_destination: &str,
    ssh_port: u16,
    tcp_port: Option<u16>,
    use_mmap: bool,
    input_files: &[PathBuf],
    prealloc: bool,
    compression: &Option<Compression>,
    start_port: u16,
    end_port: u16,
    ssh_timeout_ms: u64,
    tcp_delay_ms: u64,
    tcp_conect_mode: TcpConnectMode,
) -> Result<()> {
    log::debug!("Connecting to {remote_ip} with a timeout of {ssh_timeout_ms} ms");
    let connection_result = ssh::create_session()
        .username(username)
        .password(password)
        .private_key_path(priv_key_path)
        .connect_with_timeout(
            format!("{remote_ip}:{ssh_port}"),
            Some(Duration::from_millis(ssh_timeout_ms)),
        );

    let mut session = match connection_result {
        Ok(session) => session.run_backend(),
        Err(e) => bail!("{e}"),
    };

    let tcp_port = match tcp_port {
        Some(tp) => tp,
        None => remote_find_free_port::remote_find_free_port(&mut session, start_port, end_port)?,
    };

    log::debug!("Using TCP port: {tcp_port}");
    let remote_cmd = remote_cmd::remote_qft_command_str(
        remote_destination,
        tcp_port,
        compression.into(),
        verbosity_to_args(cfg),
        input_files.len() > 1,
    );

    log::debug!("Sending remote qft command {remote_cmd}");

    let server_ready_flag = AtomicBool::new(false);
    let server_output = std::thread::scope(|scope| {
        let server_h: ScopedJoinHandle<Result<Vec<u8>>> = scope.spawn(|| {
            let mut exec = session.open_exec()?;
            exec.send_command(&remote_cmd)?;
            let (exit_status, terminate_msg) = (exec.exit_status()?, exec.terminate_msg()?);
            log::debug!("Remote command exit status: {exit_status}");
            if !terminate_msg.is_empty() {
                log::debug!("Remote command terminate message: {terminate_msg}");
            }
            log::trace!("Sleeping {tcp_delay_ms} before allowing client to initiate transfer");
            std::thread::sleep(Duration::from_millis(tcp_delay_ms));
            server_ready_flag.store(true, Ordering::Relaxed);
            let res = exec.get_result()?;
            log::debug!("{}", String::from_utf8_lossy(&res));
            session.close();
            Ok(res)
        });

        let client_h = scope.spawn(|| {
            log::debug!("Starting client thread targetting {remote_ip}:{tcp_port}");
            log::trace!("use mmap: {use_mmap}");
            log::trace!("file(s): {input_files:?}");
            log::trace!("prealloc: {prealloc}");
            log::trace!("compression: {compression:?}");
            while !server_ready_flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(2));
            }
            crate::send::client::run_client(
                remote_ip.parse()?,
                tcp_port,
                use_mmap,
                input_files,
                prealloc,
                *compression,
                tcp_conect_mode,
            )
        });
        log::trace!("Joining client thread");
        client_h
            .join()
            .expect("Failed joining client thread")
            .unwrap();
        log::trace!("Joining server thread");
        server_h.join().expect("Failed joining server thread")
    });
    log::debug!("End");

    let server_raw_output = server_output?;
    log::debug!(
        "remote server output: {}",
        String::from_utf8(server_raw_output)?
    );

    // Close session.

    Ok(())
}
