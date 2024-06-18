use anyhow::Result;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, StdinLock},
    net::TcpStream,
    path::Path,
};

use crate::BUFFERED_RW_BUFSIZE;

pub fn file_with_bufreader(path: &Path) -> Result<BufReader<File>> {
    let f = fs::File::open(path)?;
    let reader = BufReader::with_capacity(BUFFERED_RW_BUFSIZE, f);
    Ok(reader)
}

pub fn stdin_bufreader() -> BufReader<StdinLock<'static>> {
    let stdin = io::stdin().lock();
    BufReader::with_capacity(BUFFERED_RW_BUFSIZE, stdin)
}

pub fn tcp_bufwriter(socket: &TcpStream) -> BufWriter<&TcpStream> {
    BufWriter::with_capacity(BUFFERED_RW_BUFSIZE, socket)
}
