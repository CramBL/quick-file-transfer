use anyhow::Result;
mod util;
use evaluate_compression::EvaluateCompressionArgs;
use util::*;
pub mod evaluate_compression;

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

#[derive(Debug, Args, Clone)]
pub struct ContentTransferArgs {
    /// Compression format
    #[arg(short, long)]
    compression: Option<Compression>,

    /// Supply a file for I/O (if none: use stdio)
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Client will send the size of the file to the server allowing the server to preallocate for the expected size
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    prealloc: bool,
}

impl ContentTransferArgs {
    pub fn compression(&self) -> Option<Compression> {
        self.compression
    }
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }
    pub fn prealloc(&self) -> bool {
        self.prealloc
    }
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Run in Listen (server) mode
    Listen(ListenArgs),
    /// Run in Send (client) mode
    Send(SendArgs),
    /// Use mDNS utilities
    Mdns(MdnsArgs),
    /// Evaluate which compression works best for some content
    EvaluateCompression(EvaluateCompressionArgs),
}

/// Holds the Listen subcommands
#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct ListenArgs {
    /// Host IP e.g. `127.0.0.1`
    #[arg(long, default_value_t  = String::from("0.0.0.0"))]
    pub ip: String,
    /// e.g. 30301
    #[arg(short, long, default_value_t = 12993)]
    pub port: u16,
    #[command(flatten)]
    pub content_transfer_args: ContentTransferArgs,
}

/// Holds the Send subcommands
#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub struct SendArgs {
    #[command(subcommand)]
    pub subcmd: SendCommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum SendCommand {
    /// Send to target by specifying IP e.g. `192.1.1.1`
    Ip(SendIpArgs),
    /// Send to target by specifying mDNS hostname e.g. `foo.local`
    Mdns(SendMdnsArgs),
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct SendIpArgs {
    /// IP to send to e.g. `192.0.0.1`
    pub ip: String,
    /// e.g. 12005
    #[arg(short, long, default_value_t = 12993, value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,
    /// Send a message to the server
    #[arg(short, long)]
    pub message: Option<String>,
    #[command(flatten)]
    pub content_transfer_args: ContentTransferArgs,
    /// Use memory mapping mode
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    pub mmap: bool,
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct SendMdnsArgs {
    /// mDNS hostname e.g. `foo.local`
    pub hostname: String,
    /// Maximum time (ms) to attempt to resolve IP of mDNS hostname
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
    /// Preferred IP version (attempts to fall back to another variant if the preferred version is not found)
    #[arg(long, default_value_t = IpVersion::V4)]
    pub ip_version: IpVersion,
    /// e.g. 12005
    #[arg(short, long, default_value_t = 12993)]
    pub port: u16,
    /// Send a message to the server
    #[arg(short, long)]
    pub message: Option<String>,
    #[command(flatten)]
    pub content_transfer_args: ContentTransferArgs,
    /// Use memory mapping mode
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    pub mmap: bool,
}

/// Holds the mDNS subcommands
#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
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
pub struct ServiceTypeArgs {
    /// Service label e.g. `foo` -> `_foo._<service_protocol>.local.`
    #[arg(name("service-label"), short('l'), long)]
    pub label: String,
    /// Service protocol e.g. `tcp` -> `_<service_label>._tcp.local.`
    #[arg(name = "service-protocol", long, visible_alias("proto"))]
    pub protocol: String,
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsDiscoverArgs {
    #[command(flatten)]
    pub service_type: ServiceTypeArgs,
    /// How long in ms to attempt to discover services before shutdown
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
}

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsResolveArgs {
    /// mDNS hostname to resolve e.g. `foo` (translates to `foo.local.`)
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
    #[command(flatten)]
    pub service_type: ServiceTypeArgs,
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

#[derive(Debug, Default, ValueEnum, Clone, Copy, Display, EnumIter, PartialEq)]
pub enum Compression {
    Gzip,
    Bzip2,
    Xz,
    Lz4,
    #[default]
    None,
}

#[derive(Debug, Default, ValueEnum, Clone, Copy)]
pub enum IpVersion {
    #[default]
    V4,
    V6,
}

impl fmt::Display for IpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpVersion::V4 => write!(f, "v4"),
            IpVersion::V6 => write!(f, "v6"),
        }
    }
}
