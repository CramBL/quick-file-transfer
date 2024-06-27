use super::Compression;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(flatten_help = true)]
#[cfg(feature = "ssh")]
pub struct SendSshArgs {
    /// 'classic' ssh/scp form of `<user>@<hostname>:<dst>`
    #[arg(conflicts_with("target-mode"), group("arg-mode"), value_parser = parse_scp_style)]
    pub target: Option<TargetComponents>,

    /// Remote user e.g `foo` in `foo@127.0.0.1`
    #[arg(short, long, group("arg-mode"), env = crate::ssh::ENV_REMOTE_USER)]
    pub user: Option<String>,

    /// Remote destination to transfer content to
    #[arg(short, long, visible_alias("dest"))]
    pub destination: Option<PathBuf>,

    /// mDNS hostname e.g. `foo.local`
    #[cfg(feature = "mdns")]
    #[arg(long, group("target-mode"), conflicts_with("ip"))]
    pub hostname: Option<String>,

    /// Ip for the remote
    #[arg(long, group("target-mode"))]
    pub ip: Option<String>,

    /// Port that will be used to do the transfer via TCP. Prefer leaving this value empty. If no port is specified, the remote will attempt to find a free port. Don't use this unless you have very specific needs.
    #[arg(long)]
    pub tcp_port: Option<u16>,

    /// Maximum time (ms) to attempt to resolve IP of mDNS hostname
    #[arg(long, default_value_t = 5000)]
    pub mdns_resolve_timeout_ms: u64,

    /// Maximum time (ms) to attempt to establish an SSH connection
    #[arg(long, default_value_t = 10000)]
    pub ssh_timeout_ms: u64,

    /// Time (ms) to wait from starting the server on the remote, to initiating the data transfer from the client.
    /// Use case: When the used (TCP) port is forwarded, it might be possible to initiate a data transfer before
    /// the server on the remote is listening, and thus sinkholing the data (e.g. if server is in a docker container).
    /// In such a case, waiting for a few hundred ms should be enough to reliable perform the transfer.
    #[arg(long, default_value_t = 300)]
    pub tcp_delay_ms: u64,

    /// Preferred IP version (attempts to fall back to another variant if the preferred version is not found)
    #[cfg(feature = "mdns")]
    #[arg(long, default_value_t = crate::config::misc::IpVersion::V4)]
    pub ip_version: crate::config::misc::IpVersion,

    /// Port for SSH
    #[arg(short('p'), long, default_value_t = 22)]
    pub ssh_port: u16,

    /// Compression format
    #[command(subcommand)]
    pub compression: Option<Compression>,

    /// Path to the SSH private key to use for authorization (default: looks for a key in ~/.ssh)
    #[arg(long, env(crate::ssh::ENV_SSH_PRIVATE_KEY))]
    pub ssh_private_key_path: Option<PathBuf>,

    /// Provide a path to a directory containing SSH key(s) to use for auth. Default: $HOME/.ssh on Unix and $APP_DATA/.ssh on windows
    #[arg(long, env(crate::ssh::ENV_SSH_KEY_DIR))]
    pub ssh_key_dir: Option<PathBuf>,

    /// Start of the port range to look for free ports for TCP transfer. IANA recommends: 49152-65535 for dynamic use.
    #[arg(short, long, default_value_t = 49152)]
    pub start_port: u16,

    /// end of the port range to look for free ports for TCP transfer
    #[arg(short, long, requires("start_port"), default_value_t = u16::MAX)]
    pub end_port: u16,
}

/// The components in the target args (if present) e.g. user@hostname:/home/user/f.txt
#[derive(Debug, Clone)]
pub struct TargetComponents {
    pub user: String,
    pub host: String,
    pub destination: PathBuf,
}

fn parse_scp_style(input: &str) -> Result<TargetComponents, String> {
    let parts: Vec<&str> = input.split('@').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid SSH argument format: {input}"));
    }
    let user = parts[0].to_string();
    let host_and_dest: Vec<&str> = parts[1].split(':').collect();
    if host_and_dest.len() != 2 {
        return Err(format!("Invalid SSH argument format: {input}"));
    }
    let host = host_and_dest[0].to_string();
    let destination = PathBuf::from(host_and_dest[1]);

    Ok(TargetComponents {
        user,
        host,
        destination,
    })
}
