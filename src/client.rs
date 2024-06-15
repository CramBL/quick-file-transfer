use std::{io::Write, net::TcpStream};

use crate::{config::Config, util::tcp_stream};
use anyhow::{Ok, Result};

pub fn run_client(cfg: Config) -> Result<()> {
    let mut tcp_stream = tcp_stream(cfg.address())?;

    if let Some(msg) = cfg.message() {
        let res = tcp_stream.write_all(msg.as_bytes());
        log::debug!("Wrote message: {msg}");
        log::debug!("TCP write result: {res:?}");
    }

    Ok(())
}
