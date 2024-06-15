use std::{io::Write, net::TcpStream};

use crate::{config::Config, util::tcp_listen};
use anyhow::{Ok, Result};

pub fn run_server(cfg: Config) -> Result<()> {
    let mut stream = tcp_listen(cfg.address())?;

    Ok(())
}
