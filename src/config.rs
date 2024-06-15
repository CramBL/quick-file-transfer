use anyhow::{Context, Result};
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
#[command(name = "Fast File Transfer", version, styles = cli_styles())]
#[command(bin_name = "fft")]
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
    #[clap(short, long,action = ArgAction::SetTrue, conflicts_with("verbose"), global = true, env = "FFT_QUIET")]
    pub quiet: bool,

    #[arg(short, long)]
    ip: String,

    #[arg(short, long)]
    port: u16,

    /// Send a message to the server
    #[arg(short, long)]
    message: Option<String>,
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
        Address::new(&self.ip, self.port)
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
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
    #[command(arg_required_else_help = true)]
    Listen,
    /// Run in Connect (client) mode
    #[command(arg_required_else_help = true)]
    Connect,
}
