use std::net::IpAddr;

use anyhow::Result;
use stderrlog::LogLevelNum;
use transfer::{listen::ListenArgs, send::SendArgs};
mod util;
use util::*;

pub mod compression;
pub mod transfer;

pub const BIN_NAME: &str = "qft";

#[cfg(feature = "evaluate-compression")]
pub mod evaluate_compression;
#[cfg(feature = "mdns")]
pub mod mdns;

pub mod misc;

#[derive(Debug, Parser)]
#[command(name = "Quick File Transfer", version, styles = misc::cli_styles())]
#[command(bin_name = BIN_NAME)]
pub struct Config {
    /// Accepted subcommands, e.g. `version`
    #[clap(subcommand)]
    pub command: Option<Command>,

    /// Pass many times for more log output
    ///
    /// By default, it'll report errors, warnings and info,
    /// `-v` enables debug messages, `-vv` for trace messages.
    #[arg(short, long, action = ArgAction::Count, default_value_t = 0, global = true)]
    pub verbose: u8,

    /// Silence all output
    #[arg(short, long, action = ArgAction::SetTrue, conflicts_with("verbose"), global = true, env = "QFT_QUIET")]
    pub quiet: bool,

    #[arg(
        long,
        require_equals = true,
        value_name = "WHEN",
        default_value_t = clap::ColorChoice::Auto,
        default_missing_value = "always",
        value_enum,
        global = true
    )]
    pub color: clap::ColorChoice,

    /// Generate completion scripts for the specified shell.
    /// Note: The completion script is printed to stdout
    #[arg(
        long = "completions",
        value_hint = clap::ValueHint::Other,
        value_name = "SHELL"
    )]
    pub completions: Option<clap_complete::Shell>,
}

impl Config {
    pub fn init() -> Result<Self> {
        let cfg = Self::parse();

        let log_level: LogLevelNum = match cfg.verbose {
            0 => LogLevelNum::Info,
            1 => LogLevelNum::Debug,
            255 => LogLevelNum::Off,
            _ => LogLevelNum::Trace,
        };

        let log_color_when: stderrlog::ColorChoice = match cfg.color {
            clap::ColorChoice::Auto => stderrlog::ColorChoice::Auto,
            clap::ColorChoice::Always => stderrlog::ColorChoice::Always,
            clap::ColorChoice::Never => stderrlog::ColorChoice::Never,
        };

        stderrlog::new()
            .verbosity(log_level)
            .quiet(cfg.quiet)
            .color(log_color_when)
            .init()?;

        Ok(cfg)
    }

    /// Generate completion scripts for the specified shell.
    pub fn generate_completion_script(shell: clap_complete::Shell) {
        use clap::CommandFactory;
        clap_complete::generate(
            shell,
            &mut Config::command(),
            BIN_NAME,
            &mut std::io::stdout(),
        );
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run in Listen (server) mode
    Listen(ListenArgs),
    /// Run in Send (client) mode
    Send(SendArgs),
    /// Use mDNS utilities
    #[cfg(feature = "mdns")]
    Mdns(mdns::MdnsArgs),
    /// Evaluate which compression works best for file content
    #[cfg(feature = "evaluate-compression")]
    EvaluateCompression(evaluate_compression::EvaluateCompressionArgs),
    /// Get a free port from the host OS. Optionally specify on which IP or a port range to scan for a free port.
    GetFreePort(GetFreePortArgs),
}

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct GetFreePortArgs {
    /// Host IP e.g. `127.0.0.1` for localhost
    #[arg(default_value_t  = String::from("0.0.0.0"), value_parser = valid_ip)]
    pub ip: String,

    /// Start of the port range e.g. 50000. IANA recommends: 49152-65535 for dynamic use.
    #[arg(short, long)]
    pub start_port: Option<u16>,

    /// End of the port range e.g. 51000. IANA recommends: 49152-65535 for dynamic use.
    #[arg(short, long, requires("start_port"))]
    pub end_port: Option<u16>,
}

fn valid_ip(ip_str: &str) -> Result<String, String> {
    if ip_str.parse::<IpAddr>().is_err() {
        return Err(format!("'{ip_str}' is not a valid IP address."));
    }
    Ok(ip_str.to_owned())
}
