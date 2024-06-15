use anyhow::{Ok, Result};
use std::fmt;
use std::net::{TcpListener, TcpStream};

#[derive(Debug, Clone)]
pub struct Address<'cfg> {
    pub ip: &'cfg str,
    pub port: u16,
}

impl<'cfg> Address<'cfg> {
    pub fn new(ip: &'cfg str, port: u16) -> Self {
        Self { ip, port }
    }
}

impl<'cfg> fmt::Display for Address<'cfg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

pub fn tcp_stream(addr: Address) -> Result<TcpStream> {
    let streamer = TcpStream::connect(addr.to_string())?;
    Ok(streamer)
}

pub fn tcp_listen(addr: Address) -> Result<TcpListener> {
    let listener = TcpListener::bind(addr.to_string())?;
    Ok(listener)
}
