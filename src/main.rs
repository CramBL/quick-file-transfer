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
use client::run_client;
use config::Config;
use server::run_server;

pub mod client;
pub mod config;
pub mod mmap_reader;
pub mod server;
pub mod util;

pub const TCP_STREAM_BUFSIZE: usize = 2 * 1024;
pub const BUFFERED_RW_BUFSIZE: usize = 32 * 1024;

fn main() -> Result<()> {
    let cfg = Config::init()?;

    log::trace!("{cfg:?}");
    log::debug!("{:?}", cfg.address());

    match cfg.command {
        config::Command::Listen => run_server(&cfg)?,
        config::Command::Connect => run_client(&cfg)?,
    }

    Ok(())
}
