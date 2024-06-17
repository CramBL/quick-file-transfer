use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{command, ArgAction, Args, Parser, Subcommand, ValueEnum};

use crate::util::Address;

/// Styling for the `help` terminal output
pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Debug, Parser)]
#[command(name = "Quick File Transfer", version, styles = cli_styles())]
#[command(bin_name = "qft")]
pub struct Config {
    /// Accepted subcommands, e.g. `version`
    #[clap(subcommand)]
    pub command: Command,

    /// Pass many times for more log output
    ///
    /// By default, it'll report errors, warnings and info,
    /// `-v` enables debug messages, `-vv` for trace messages.
    #[arg(short, long, action = ArgAction::Count, default_value_t = 0, global = true)]
    pub verbose: u8,

    /// Silence all output
    #[arg(short, long, action = ArgAction::SetTrue, conflicts_with("verbose"), global = true, env = "QFT_QUIET")]
    pub quiet: bool,

    /// e.g. 127.0.0.1
    #[arg(short, long, group = "network-address", global(true))]
    ip: Option<String>,

    /// e.g. 8080
    /// TODO: default port? and/or option to retrieve a randomly generated free port
    #[arg(short, long)]
    port: Option<u16>,

    /// Send a message to the server
    #[arg(short, long)]
    message: Option<String>,

    /// Supply a file for I/O (if none: use stdio)
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Compression format
    #[arg(short, long)]
    compression: Option<Compression>,

    /// Use memory mapping mode
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    mmap: bool,

    /// Client will send the size of the file to the server allowing the server to preallocate for the expected size
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    prealloc: bool,

    /// Use mDNS instead of an IP
    #[arg(
        long,
        conflicts_with("ip"),
        requires = "port",
        group = "network-address"
    )]
    mdns: Option<String>,
}

impl Config {
    pub fn init() -> Result<Self> {
        let cfg = Self::parse();

        use stderrlog::LogLevelNum;
        let log_level: LogLevelNum = match cfg.verbose {
            0 => LogLevelNum::Info,
            1 => LogLevelNum::Debug,
            255 => LogLevelNum::Off,
            _ => LogLevelNum::Trace,
        };

        stderrlog::new()
            .verbosity(log_level)
            .quiet(cfg.quiet)
            .init()?;

        Ok(cfg)
    }

    pub fn address(&self) -> Address {
        if let Some(ip) = self.ip.as_deref() {
            Address::new(ip, self.port.unwrap())
        } else {
            todo!("Convert mDNS hostname to IP")
        }
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    pub fn compression(&self) -> Option<Compression> {
        self.compression
    }

    pub fn use_mmap(&self) -> bool {
        self.mmap
    }

    pub fn prealloc(&self) -> bool {
        self.prealloc
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum ColorWhen {
    Always,
    Auto,
    Never,
}

impl std::fmt::Display for ColorWhen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run in listen (server) mode
    Listen,
    /// Run in Connect (client) mode
    Connect,
    /// Use mDNS utilities
    Mdns(MdnsArgs),
}

/// Holds the mDNS subcommands
#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true)]
#[command(arg_required_else_help = true)]
pub struct MdnsArgs {
    #[command(subcommand)]
    pub subcmd: MdnsCommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum MdnsCommand {
    /// Discover mDNS
    Discover(MdnsDiscoverArgs),
    /// Resolve mDNS hostname
    Resolve(MdnsResolveArgs),
    /// Register a temporary service (for testing)
    Register(MdnsRegisterArgs),
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsDiscoverArgs {
    /// Service label e.g. `foo` -> `_foo._<service_protocol>.local.`
    #[arg(short('l'), long)]
    pub service_label: String,
    /// Service protocol e.g. `tcp` -> `_<service_label>._tcp.local.`
    #[arg(long, visible_alias("proto"))]
    pub service_protocol: String,
    /// How long in ms to attempt to discover services before shutdown
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsResolveArgs {
    /// mDNS hostname to resolve e.g. `foo` (translates to `foo.local.`)
    #[arg(short('n'), long)]
    pub hostname: String,
    /// Sets a timeout in milliseconds (default 10s)
    #[arg(long, default_value_t = 10000)]
    pub timeout_ms: u64,
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsRegisterArgs {
    /// Service name to register e.g. `foo` (translates to `foo.local.`)
    #[arg(short('n'), long, default_value_t = String::from("test_name"))]
    pub hostname: String,
    /// Service label e.g. `foo` -> `_foo._<service_protocol>.local.`
    #[arg(short('l'), long, default_value_t = String::from("test_label"))]
    pub service_label: String,
    /// Service protocol e.g. `tcp` -> `_<service_label>._tcp.local.`
    #[arg(short('t'), long, default_value_t = String::from("udp"), visible_alias = "proto")]
    pub service_protocol: String,
    #[arg(short, long, default_value_t = String::from("test_inst"))]
    pub instance_name: String,
    /// How long to keep it alive in ms
    #[arg(long, default_value_t = 600000)]
    pub keep_alive_ms: u64,
    /// Service IP, if none provided -> Use auto adressing
    #[arg(long)]
    pub ip: Option<String>,
    /// Service port
    #[arg(long, default_value_t = 11542)]
    pub port: u16,
}

use strum_macros::Display;
#[derive(Debug, Default, ValueEnum, Clone, Copy, Display)]
pub enum Compression {
    Gzip,
    Bzip2,
    Xz,
    Lz4,
    #[default]
    None,
}
