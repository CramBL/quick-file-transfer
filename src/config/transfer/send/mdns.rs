use clap::Args;

use crate::config::misc::IpVersion;

use super::Compression;

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
