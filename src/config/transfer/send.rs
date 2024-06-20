use crate::config::{util::*, IpVersion};

use super::ContentTransferArgs;

/// Holds the Send subcommands
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub struct SendArgs {
    #[command(subcommand)]
    pub subcmd: SendCommand,
}

#[derive(Subcommand, Debug)]
pub enum SendCommand {
    /// Send to target by specifying IP e.g. `192.1.1.1`
    Ip(SendIpArgs),
    /// Send to target by specifying mDNS hostname e.g. `foo.local`
    Mdns(SendMdnsArgs),
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
    #[command(flatten)]
    pub content_transfer_args: ContentTransferArgs,
    /// Compression format
    #[command(subcommand)]
    pub compression: Option<Compression>,
    /// Use memory mapping mode
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    pub mmap: bool,
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
    #[command(flatten)]
    pub content_transfer_args: ContentTransferArgs,
    /// Compression format
    #[command(subcommand)]
    pub compression: Option<Compression>,
    /// Use memory mapping mode
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    pub mmap: bool,
}
