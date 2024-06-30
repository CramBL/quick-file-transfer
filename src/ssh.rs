use crate::{
    config::{
        compression::Compression,
        transfer::{send::ssh::SendSshArgs, util::TcpConnectMode},
        Config,
    },
    util::verbosity_to_args,
};
use anyhow::Result;
use remote_session::RemoteSshSession;
use std::{
    path::{Path, PathBuf},
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
pub mod remote_session;
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

    run_ssh(
        cfg,
        remote_info.user,
        ssh_private_key_path.as_deref(),
        ssh_key_dir.as_deref(),
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
pub fn run_ssh(
    cfg: &Config,
    username: &str,
    private_key: Option<&Path>,
    private_key_dir: Option<&Path>,
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
    let mut session = RemoteSshSession::new(
        username,
        (remote_ip, ssh_port),
        Some(Duration::from_millis(ssh_timeout_ms)),
        private_key,
        private_key_dir,
    )?;

    let tcp_port = match tcp_port {
        Some(tp) => tp,
        None => session.find_free_port(start_port, end_port)?,
    };

    log::debug!("Using TCP port: {tcp_port}");
    let remote_cmd = remote_cmd::remote_qft_command_str(
        remote_destination,
        tcp_port,
        verbosity_to_args(cfg),
        input_files.len() > 1,
    );

    log::debug!("Sending remote qft command {remote_cmd}");

    let server_ready_flag = AtomicBool::new(false);
    let server_output = std::thread::scope(|scope| {
        let server_h: ScopedJoinHandle<Result<Vec<u8>>> = scope.spawn(|| {
            session.run_cmd(&remote_cmd)?;

            log::trace!("Sleeping {tcp_delay_ms} before allowing client to initiate transfer");
            std::thread::sleep(Duration::from_millis(tcp_delay_ms));
            server_ready_flag.store(true, Ordering::Relaxed);
            let out = session
                .get_cmd_output()
                .expect("No command output for remote sesion");
            session.close();
            Ok(out)
        });

        let client_h = scope.spawn(|| {
            log::debug!("Starting client thread targetting {remote_ip}:{tcp_port}");
            log::trace!(
                "\
            use mmap: {use_mmap}\
            \nfile(s): {input_files:?}\
            \nprealloc: {prealloc}\
            \ncompression: {compression:?}"
            );
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

    log::debug!(
        "remote server output: {}",
        String::from_utf8_lossy(&server_output?)
    );

    // Close session.

    Ok(())
}
