use anyhow::Result;
use evaluate_compression::EvaluateCompressionArgs;
use transfer::{send::SendArgs, ContentTransferArgs};
mod util;
use util::*;
pub mod compression;
pub mod evaluate_compression;
pub mod transfer;

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

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Run in Listen (server) mode
    Listen(ListenArgs),
    /// Run in Send (client) mode
    Send(SendArgs),
    /// Use mDNS utilities
    Mdns(MdnsArgs),
    /// Evaluate which compression works best for file content
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
    /// Exit as soon as the first IP of the specified hostname has been resolved
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub short_circuit: bool,
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
