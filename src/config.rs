use std::net::IpAddr;

use anyhow::Result;
use stderrlog::LogLevelNum;
use transfer::{listen::ListenArgs, send::SendArgs};
mod util;
use util::*;

pub mod compression;
pub mod transfer;

#[cfg(feature = "evaluate-compression")]
pub mod evaluate_compression;
#[cfg(feature = "mdns")]
pub mod mdns;

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
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorWhen {
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
    GetFreePort(GetFreePortArgs),
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

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct GetFreePortArgs {
    /// Host IP e.g. `127.0.0.1` for localhost
    #[arg(default_value_t  = String::from("0.0.0.0"), value_parser = valid_ip)]
    pub ip: String,
}

fn valid_ip(ip_str: &str) -> Result<String, String> {
    if ip_str.parse::<IpAddr>().is_err() {
        return Err(format!("'{ip_str}' is not a valid IP address."));
    }
    Ok(ip_str.to_owned())
}
