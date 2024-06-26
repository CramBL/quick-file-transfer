use crate::{
    config::{
        compression::Compression,
        transfer::send::ssh::{SendSshArgs, TargetComponents},
        Config,
    },
    util::verbosity_to_args,
};
use anyhow::{bail, Result};
use std::{
    borrow::Cow,
    ffi::OsStr,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    thread::ScopedJoinHandle,
    time::Duration,
};

#[cfg(feature = "mdns")]
mod mdns_util;
pub mod private_key;
mod remote_cmd;
pub(crate) mod util;

pub const ENV_REMOTE_PASSWORD: &str = "QFT_REMOTE_PASSWORD";
pub const ENV_SSH_KEY_DIR: &str = "QFT_SSH_KEY_DIR";
pub const ENV_SSH_PRIVATE_KEY: &str = "QFT_SSH_PRIVATE_KEY";

pub const ENV_REMOTE_USER: &str = "QFT_REMOTE_USER";

#[derive(Debug, Clone, Copy)]
enum Remote<'a> {
    Ip(&'a str),
    #[cfg(feature = "mdns")]
    MdnsHostname(&'a str),
}

impl<'a> Remote<'a> {
    pub fn new(host: &'a str) -> Result<Self> {
        if host.parse::<std::net::IpAddr>().is_ok() {
            return Ok(Self::Ip(host));
        }
        #[cfg(feature = "mdns")]
        if mdns_util::is_mdns_hostname(host) {
            return Ok(Self::MdnsHostname(host));
        }
        bail!("'{host}' is not an IP or a mDNS/DNS-SD hostname");
    }

    #[cfg(feature = "mdns")]
    pub fn to_resolved_ip_str(self, timeout_ms: u64) -> Result<Cow<'a, str>> {
        match self {
            Remote::Ip(ip) => Ok(Cow::Borrowed(ip)),
            Remote::MdnsHostname(hn) => {
                let ip = mdns_util::get_remote_ip_from_hostname(
                    hn,
                    timeout_ms,
                    crate::config::IpVersion::V4,
                )?;
                let ip_str = ip.to_string().into();
                Ok(ip_str)
            }
        }
    }

    #[cfg(not(feature = "mdns"))]
    pub fn to_ip_str(self) -> Cow<'a, str> {
        debug_assert!(matches!(self, Remote::Ip(_)));
        match self {
            Remote::Ip(ip) => Cow::Borrowed(ip),
        }
    }
}

struct RemoteInfo<'a> {
    user: &'a str,
    ssh_port: u16,
    resolved_ip: Cow<'a, str>,
}

impl<'a> RemoteInfo<'a> {
    pub fn new(user: &'a str, ssh_port: u16, resolved_ip: Cow<'a, str>) -> Self {
        Self {
            user,
            ssh_port,
            resolved_ip,
        }
    }

    pub fn from_args(ssh_args: &'a SendSshArgs) -> Result<Self> {
        let mut user: Option<&str> = None;
        let mut remote: Option<Remote> = None;
        if let Some(TargetComponents {
            user: ref user_component,
            ref host,
            destination: _,
        }) = ssh_args.target
        {
            user = Some(user_component.as_str());
            remote = Some(Remote::new(host)?);
        };

        if let Some(ref u) = ssh_args.user {
            debug_assert!(user.is_none());
            user = Some(u);
        }
        #[cfg(feature = "mdns")]
        if let Some(ref h) = ssh_args.hostname {
            debug_assert!(remote.is_none());
            remote = Some(Remote::new(h)?);
        }
        if let Some(ref ip) = ssh_args.ip {
            debug_assert!(remote.is_none());
            remote = Some(Remote::Ip(ip));
        }
        #[cfg(feature = "mdns")]
        let resolved_ip = remote.unwrap().to_resolved_ip_str(ssh_args.timeout_ms)?;
        #[cfg(not(feature = "mdns"))]
        let resolved_ip = remote.unwrap().to_ip_str();

        Ok(Self::new(user.unwrap(), ssh_args.ssh_port, resolved_ip))
    }
}

pub fn handle_send_ssh(
    cfg: &Config,
    args: &SendSshArgs,
    input_file: Option<&Path>,
    prealloc: bool,
    use_mmap: bool,
) -> Result<()> {
    let remote_info = RemoteInfo::from_args(args)?;
    let SendSshArgs {
        user: _,
        #[cfg(feature = "mdns")]
            hostname: _,
        timeout_ms: _,
        ip_version: _,
        ssh_port: _,
        compression,
        ip: _,
        target: _,
        destination,
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
        destination.as_deref(),
        remote_info.ssh_port,
        *tcp_port,
        use_mmap,
        input_file,
        prealloc,
        compression,
        *start_port,
        *end_port,
        *ssh_timeout_ms,
        *tcp_delay_ms,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_ssh(
    cfg: &Config,
    (username, password): (&str, &str),
    priv_key_path: &OsStr,
    remote_ip: &str,
    remote_destination: Option<&Path>,
    ssh_port: u16,
    tcp_port: Option<u16>,
    use_mmap: bool,
    input_file: Option<&Path>,
    prealloc: bool,
    compression: &Option<Compression>,
    start_port: u16,
    end_port: u16,
    ssh_timeout_ms: u64,
    tcp_delay_ms: u64,
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
        None => {
            const GET_FREE_PORT_CMD_PREFIX: &str = "qft get-free-port";
            const START_PORT_OPTION: &str = "--start-port";
            const END_PORT_OPTION: &str = "--end-port";
            let get_free_port_cmd = format!("{GET_FREE_PORT_CMD_PREFIX} {START_PORT_OPTION} {start_port} {END_PORT_OPTION} {end_port} -q",            );
            log::debug!(
                "No TCP port specified, querying remote for a free port with '{get_free_port_cmd}'"
            );
            let mut exec = session.open_exec()?;
            exec.send_command(&get_free_port_cmd)?;
            let exit_status = exec.exit_status()?;
            let terminate_msg = exec.terminate_msg()?;
            log::debug!("Exit status: {exit_status}");
            if !terminate_msg.is_empty() {
                log::debug!("Terminate message: {exit_status}");
            }
            let raw_out = exec.get_result()?;
            log::trace!("Receivied raw output {raw_out:?}");
            log::trace!(
                "Receivied output as lossy utf8:{}",
                String::from_utf8_lossy(&raw_out)
            );
            // Take the first N-bytes that are ascii digits and parse them to u16
            let free_port = raw_out
                .iter()
                .take_while(|&&byte| byte.is_ascii_digit())
                .fold(String::new(), |mut acc, &byte| {
                    acc.push(byte as char);
                    acc
                })
                .parse::<u16>()
                .expect("Failed to parse u16");
            log::trace!(
                "'{get_free_port_cmd}' output as utf8: {}",
                String::from_utf8_lossy(&raw_out)
            );
            free_port
        }
    };

    log::debug!("Using TCP port: {tcp_port}");
    let remote_cmd = remote_cmd::remote_qft_command_str(
        remote_destination.map(|p| p.to_str().unwrap()),
        tcp_port,
        prealloc,
        compression.into(),
        verbosity_to_args(cfg),
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
            log::trace!("file: {input_file:?}");
            log::trace!("prealloc: {prealloc}");
            log::trace!("compression: {:?}", compression);
            while !server_ready_flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(2));
            }
            crate::send::client::run_client(
                remote_ip.parse()?,
                tcp_port,
                None,
                use_mmap,
                input_file,
                prealloc,
                *compression,
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
