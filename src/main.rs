// Performance lints
#![warn(variant_size_differences)]
#![warn(
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    clippy::mutex_integer,
    clippy::mem_forget,
    clippy::maybe_infinite_iter
)]

use anyhow::Result;
use config::{Command, Config};
use evaluate_compression::evaluate_compression;
use mdns::handle_mdns_command;
use send::handle_send_cmd;
use server::listen;

pub mod config;
pub mod evaluate_compression;
pub mod mdns;
pub mod mmap_reader;
pub mod send;
pub mod server;
pub mod util;

pub const TCP_STREAM_BUFSIZE: usize = 2 * 1024;
pub const BUFFERED_RW_BUFSIZE: usize = 32 * 1024;

fn main() -> Result<()> {
    let cfg = Config::init()?;

    log::trace!("{cfg:?}");

    match cfg.command {
        Command::Listen(ref args) => listen(&cfg, args),
        Command::Send(ref cmd) => handle_send_cmd(cmd, &cfg),
        Command::Mdns(cmd) => handle_mdns_command(cmd.subcmd),
        Command::EvaluateCompression(args) => evaluate_compression(args),
    }
}
