use anyhow::Result;
use client::run_client;
use config::Config;
use server::run_server;

pub mod client;
pub mod config;
pub mod mmap_reader;
pub mod server;
pub mod util;

pub const TCP_STREAM_BUFSIZE: usize = 1024 * 4;

fn main() -> Result<()> {
    let cfg = Config::init()?;

    log::debug!("{:?}", cfg.address());

    match cfg.command {
        config::Command::Listen => run_server(cfg)?,
        config::Command::Connect => run_client(cfg)?,
    }

    Ok(())
}
