use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{command, ArgAction, Parser, Subcommand, ValueEnum};

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
    #[clap(short, long, action = ArgAction::SetTrue, conflicts_with("verbose"), global = true, env = "QFT_QUIET")]
    pub quiet: bool,

    /// e.g. 127.0.0.1
    #[arg(short, long)]
    ip: String,

    /// e.g. 8080
    #[arg(short, long)]
    port: u16,

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
