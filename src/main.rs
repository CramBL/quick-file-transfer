use anyhow::Result;
use client::run_client;
use config::Config;
use server::run_server;

pub mod client;
pub mod config;
pub mod server;
pub mod util;

fn main() -> Result<()> {
    let cfg = Config::init()?;

    dbg!(&cfg);

    log::info!("{:?}", cfg.address());

    match cfg.command {
        config::Command::Listen => run_server(cfg)?,
        config::Command::Connect => run_client(cfg)?,
    }

    Ok(())
}
