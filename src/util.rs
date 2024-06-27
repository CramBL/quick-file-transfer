use crate::config::Config;
use anyhow::Result;
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
#[must_use]
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
        1_048_577..=GI_B_VAL => {
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
/// Bind to port 0 on `ip`, which tells the OS to assign any available port, then
/// retrieve the socket address from the listener.
pub fn get_free_port(ip: &str) -> Option<u16> {
    if let Ok(listener) = TcpListener::bind((ip, 0)) {
        if let Ok(local_addr) = listener.local_addr() {
            return Some(local_addr.port());
        }
    }
    None
}

/// see more: https://www.rfc-editor.org/rfc/rfc6335.html#section-6
pub const IANA_RECOMMEND_DYNAMIC_PORT_RANGE_START: u16 = 49152;
pub const IANA_RECOMMEND_DYNAMIC_PORT_RANGE_END: u16 = 65535;

/// Bind to any available port within the specified range on `ip`,
/// then retrieve the socket address from the listener.
///
///
/// # Note
///
/// Internet Assigned Numbers Authority (IANA) suggests 49152-65535 for dynamic/ephemeral use.
/// Although note that Linux distros often use 32768-61000 so a conservative/robust range of
/// 49152-61000 is preferable.
///
/// see more: https://www.rfc-editor.org/rfc/rfc6335.html#section-6
pub fn get_free_port_in_range(ip: &str, start_port: u16, end_port: u16) -> Option<u16> {
    for port in start_port..=end_port {
        if let Ok(listener) = TcpListener::bind((ip, port)) {
            if let Ok(local_addr) = listener.local_addr() {
                return Some(local_addr.port());
            }
        }
    }
    None
}

/// Converts the verbosity from the config back to the command-line arguments that would produce that verbosity
pub fn verbosity_to_args(cfg: &Config) -> &str {
    if cfg.quiet {
        "-q"
    } else {
        match cfg.verbose {
            1 => "-v",
            2 => "-vv",
            // Verbose is not set
            _ => "",
        }
    }
}

/// This is for generating pseudo-random number for application client/server hand shake.
///
/// It is adapted from: https://docs.rs/rand_xoshiro/latest/src/rand_xoshiro/splitmix64.rs.html
/// ... and gutted
///
/// There's no strong requirement for this random number other than being fast, and lets not add the rand crate as a dependency just for this...
pub(crate) mod tiny_rnd {

    /// Get a "random" number from a seed (one shot).
    ///
    /// # Note
    ///
    /// Adapted (gutted) from: https://docs.rs/rand_xoshiro/latest/src/rand_xoshiro/splitmix64.rs.html
    ///
    /// Stateless  gutted splitmix64 random number generator.
    ///
    /// The gutted splitmix algorithm is NOT suitable for cryptographic purposes, but is
    /// very fast and has a 64 bit state.
    pub fn rnd_u32(seed: u64) -> u32 {
        const PHI: u64 = 0x9e3779b97f4a7c15;
        let mut z = seed.wrapping_add(PHI);
        // David Stafford's
        // (http://zimbry.blogspot.com/2011/09/better-bit-mixing-improving-on.html)
        // "Mix4" variant of the 64-bit finalizer in Austin Appleby's
        // MurmurHash3 algorithm.
        z = (z ^ (z >> 33)).wrapping_mul(0x62A9D9ED799705F5);
        z = (z ^ (z >> 28)).wrapping_mul(0xCB24D0A5C88C35B3);
        (z >> 32) as u32
    }
}
