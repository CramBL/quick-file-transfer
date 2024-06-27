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
}

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
