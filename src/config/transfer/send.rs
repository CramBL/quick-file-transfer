use crate::config::{util::*, IpVersion};

/// Holds the Send subcommands
#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct SendArgs {
    #[command(subcommand)]
    pub subcmd: SendCommand,

    /// Supply a file for I/O (if none: use stdio)
    #[arg(
        short,
        long,
        global(true),
        value_name("FILE"),
        group("io-content"),
        name("INPUT_FILE")
    )]
    pub file: Option<PathBuf>,

    /// Client will send the size of the file to the server allowing the server to preallocate for the expected size
    #[arg(long, action = ArgAction::SetTrue, requires = "INPUT_FILE", global(true))]
    pub prealloc: bool,

    /// Use memory mapping mode
    #[arg(long, action = ArgAction::SetTrue, requires = "INPUT_FILE", global(true))]
    pub mmap: bool,
}

#[derive(Subcommand, Debug)]
pub enum SendCommand {
    /// Send to target by specifying IP e.g. `192.1.1.1`
    Ip(SendIpArgs),

    /// Send to target by specifying mDNS hostname e.g. `foo.local`
    #[cfg(feature = "mdns")]
    Mdns(SendMdnsArgs),

    #[cfg(feature = "ssh")]
    Ssh(SendSshArgs),
}

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct SendIpArgs {
    /// IP to send to e.g. `192.0.0.1`
    pub ip: String,
    /// e.g. 12005
    #[arg(short, long, default_value_t = 12993, value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,
    /// Send a message to the server
    #[arg(short, long)]
    pub message: Option<String>,
    /// Compression format
    #[command(subcommand)]
    pub compression: Option<Compression>,
}

#[derive(Debug, Args)]
#[command(flatten_help = true)]
pub struct SendMdnsArgs {
    /// mDNS hostname e.g. `foo.local`
    pub hostname: String,
    /// Maximum time (ms) to attempt to resolve IP of mDNS hostname
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
    /// Preferred IP version (attempts to fall back to another variant if the preferred version is not found)
    #[arg(long, default_value_t = IpVersion::V4)]
    pub ip_version: IpVersion,
    /// e.g. 12005
    #[arg(short, long, default_value_t = 12993)]
    pub port: u16,
    /// Send a message to the server
    #[arg(short, long)]
    pub message: Option<String>,
    /// Compression format
    #[command(subcommand)]
    pub compression: Option<Compression>,
}

#[derive(Debug, Args)]
#[command(flatten_help = true)]
#[cfg(feature = "ssh")]
pub struct SendSshArgs {
    #[arg(conflicts_with("target-mode"), group("arg-mode"))]
    /// 'classic' ssh/scp form of <user>@<hostname>:<dst>
    pub target: Option<String>,
    /// Remote user e.g `foo` in `foo@127.0.0.1`
    #[arg(short, long, group("arg-mode"), env = crate::ssh::ENV_REMOTE_USER)]
    pub user: Option<String>,
    #[arg(short, long, visible_alias("dest"))]
    pub destination: Option<PathBuf>,
    /// mDNS hostname e.g. `foo.local`
    #[cfg(feature = "mdns")]
    #[arg(long, group("target-mode"), conflicts_with("ip"))]
    pub hostname: Option<String>,
    /// Ip for the remote
    #[arg(long, group("target-mode"), conflicts_with("hostname"))]
    pub ip: Option<String>,
    /// Port that will be used to do the transfer via TCP. If no port is specified, the remote will attempt to find a free port.
    #[arg(long)]
    pub tcp_port: Option<u16>,
    /// Maximum time (ms) to attempt to resolve IP of mDNS hostname
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
    /// Preferred IP version (attempts to fall back to another variant if the preferred version is not found)
    #[arg(long, default_value_t = IpVersion::V4, requires("hostname"))]
    pub ip_version: IpVersion,
    /// e.g. 12005
    #[arg(short, long, default_value_t = 12993)]
    pub port: u16,
    /// Compression format
    #[command(subcommand)]
    pub compression: Option<Compression>,
}
