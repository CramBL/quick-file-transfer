use anyhow::{Ok, Result};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::{fmt, fs, io};

#[derive(Debug, Clone, Copy)]
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

pub fn connect_tcp_stream(addr: Address) -> Result<TcpStream> {
    let stream = TcpStream::connect(addr.to_string())?;
    Ok(stream)
}

pub fn bind_tcp_listener(addr: Address) -> Result<TcpListener> {
    let listener = TcpListener::bind(addr.to_string())?;
    Ok(listener)
}

/// Format a value to a human readable byte magnitude description
pub fn format_data_size(size_bytes: u64) -> String {
    const KI_B_VAL: u64 = 1024;
    const KI_B_DIVIDER: f64 = 1024_f64;
    const MI_B_VAL: u64 = 1024 * KI_B_VAL;
    const MI_B_DIVIDER: f64 = MI_B_VAL as f64;
    const GI_B_VAL: u64 = 1024 * MI_B_VAL;
    const GI_B_DIVIDER: f64 = GI_B_VAL as f64;
    match size_bytes {
        0..=KI_B_VAL => {
            format!("{size_bytes:.2} B")
        }
        1025..=MI_B_VAL => {
            let kib_bytes = size_bytes as f64 / KI_B_DIVIDER;
            format!("{kib_bytes:.2} KiB")
        }
        1048577..=GI_B_VAL => {
            let mib_bytes = size_bytes as f64 / MI_B_DIVIDER;
            format!("{mib_bytes:.2} MiB")
        }
        _ => {
            let gib_bytes = size_bytes as f64 / GI_B_DIVIDER;
            format!("{gib_bytes:.2} GiB")
        }
    }
}

pub fn incremental_rw<const BUFSIZE: usize>(
    stream_writer: &mut dyn io::Write,
    reader: &mut dyn io::Read,
) -> Result<u64> {
    let mut buf = [0; BUFSIZE];
    let mut total_read = 0;
    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        total_read += bytes_read;

        let written_bytes = stream_writer.write(&buf[..bytes_read])?;
        debug_assert_eq!(
            bytes_read, written_bytes,
            "Mismatch between bytes read/written, read={bytes_read}, written={written_bytes}"
        );
    }
    Ok(total_read as u64)
}

pub fn create_file_with_len(path: &Path, len: u64) -> Result<()> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    file.set_len(len)?;
    Ok(())
}
