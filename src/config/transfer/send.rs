use std::time::Duration;

use crate::{config::util::*, util::IANA_RECOMMEND_DYNAMIC_PORT_RANGE_START};

#[cfg(feature = "mdns")]
pub mod mdns;
#[cfg(feature = "ssh")]
pub mod ssh;

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

    /// Poll the server with a specified interval (ms) until a connection is established.
    #[arg(
        long("poll"),
        global(true),
        value_name("INTERVAL_MS"),
        default_value_t = 100
    )]
    pub poll: u32,

    /// Disable polling, exit immediately if the first attempt at establishing a connection to the server fails
    #[arg(
        long("one-shot"),
        visible_alias("disable-poll"),
        default_value_t = false
    )]
    pub one_shot: bool,

    /// Maxiumum time to attempt to establish a TCP connection to remote.
    #[arg(long, default_value = Some("5000"), group("tcp_about_condition"))]
    pub tcp_timeout_ms: Option<u32>,

    /// Maximum attempts to establish a TCP connection to remote.
    #[arg(long, group("tcp_about_condition"))]
    pub tcp_max_attempts: Option<u32>,
}

impl SendArgs {
    /// Returns the configured mode for making Tcp connections
    pub fn tcp_connect_mode(&self) -> TcpConnectMode {
        if self.one_shot {
            TcpConnectMode::OneShot
        } else {
            let abort_cond = if let Some(attempts) = self.tcp_max_attempts {
                PollAbortCondition::Attempts(attempts)
            } else {
                PollAbortCondition::Timeout(Duration::from_millis(
                    self.tcp_timeout_ms.unwrap().into(),
                ))
            };
            TcpConnectMode::poll_from_ms(self.poll, abort_cond)
        }
    }
}

#[allow(clippy::large_enum_variant)] // This lint should be revised when command-line args are fairly stabilized
#[derive(Subcommand, Debug)]
pub enum SendCommand {
    /// Send to target by specifying IP e.g. `192.1.1.1`
    Ip(SendIpArgs),

    /// Send to target by specifying mDNS hostname e.g. `foo.local`
    #[cfg(feature = "mdns")]
    Mdns(mdns::SendMdnsArgs),

    /// SCP-like - Send to a target that might not have qft actively listening, authenticating over SSH and transferring over TCP.
    #[cfg(feature = "ssh")]
    #[command(long_about("SCP-like transfer to a remote target that might not have qft actively listening.\n\
    Authentication uses SSH (key based auth only) and while the transfer occurs over TCP, UNENCRYPTED!.\n\
    Just like the rest of QTF, this is not suitable for transforring sensitive information."))]
    Ssh(ssh::SendSshArgs),
}

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct SendIpArgs {
    /// IP to send to e.g. `192.0.0.1`
    pub ip: String,
    /// e.g. 49152. IANA recommends: 49152-65535 for dynamic use.
    #[arg(short, long, default_value_t = IANA_RECOMMEND_DYNAMIC_PORT_RANGE_START, value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,
    /// Compression format
    #[command(subcommand)]
    pub compression: Option<Compression>,
}
