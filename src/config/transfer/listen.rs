use crate::config::{compression::CompressionVariant, util::*};

use super::ContentTransferArgs;

/// Holds the Listen subcommands
#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct ListenArgs {
    /// Host IP e.g. `127.0.0.1`
    #[arg(long, default_value_t  = String::from("0.0.0.0"))]
    pub ip: String,
    /// e.g. 30301
    #[arg(short, long, default_value_t = 12993)]
    pub port: u16,
    #[command(flatten)]
    pub content_transfer_args: ContentTransferArgs,

    /// Compression format of the received file
    #[command(subcommand)]
    pub compression: Option<CompressionVariant>,
}
