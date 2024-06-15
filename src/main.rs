use anyhow::Result;
use config::Config;

pub mod client;
pub mod config;
pub mod server;
pub mod util;

fn main() -> Result<()> {
    let cfg = Config::init()?;

    dbg!(&cfg);

    log::info!("Port: {:?}", cfg.address());

    Ok(())
}
