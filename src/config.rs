use anyhow::Result;
use stderrlog::LogLevelNum;
use transfer::{listen::ListenArgs, send::SendArgs};
mod util;
use util::*;

pub mod compression;
#[cfg(feature = "ssh")]
pub mod ssh;
pub mod transfer;

pub const BIN_NAME: &str = "qft";

#[cfg(feature = "evaluate-compression")]
pub mod evaluate_compression;
pub mod get_free_port;
#[cfg(feature = "mdns")]
pub mod mdns;
pub mod misc;

#[derive(Debug, Parser)]
#[command(name = "Quick File Transfer", version, styles = misc::cli_styles())]
#[command(bin_name = BIN_NAME)]
pub struct Config {
    /// Accepted subcommands, e.g. `listen`
    #[clap(subcommand)]
    pub command: Option<Command>,

    /// Pass many times for more log output
    ///
    /// By default, it'll report errors, warnings and info,
    /// `-v` enables debug messages, `-vv` for trace messages.
    #[arg(short, long, action = ArgAction::Count, default_value_t = 0, global = true)]
    pub verbose: u8,

    /// Silence all log output, this will lead to better performance.
    #[arg(short, long, action = ArgAction::SetTrue, conflicts_with("verbose"), global = true, env = "QFT_QUIET")]
    pub quiet: bool,

    /// Use io_uring
    #[arg(long("io-uring"), action = ArgAction::SetTrue, global(true), env = "QFT_IO_URING")]
    pub io_uring: bool,

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

        set_tracing(&log_level);
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
    GetFreePort(get_free_port::GetFreePortArgs),
    /// SCP-like - Send to a target that might not have qft actively listening, authenticating over SSH and transferring over TCP.
    #[cfg(feature = "ssh")]
    #[command(long_about("SCP-like transfer to a remote target that might not have qft actively listening.\n\
    Authentication uses SSH (key based auth only) and while the transfer occurs over TCP, UNENCRYPTED!.\n\
    Just like the rest of QTF, this is not suitable for transforring sensitive information."))]
    Ssh(ssh::SendSshArgs),
}

fn set_tracing(_trace_level: &LogLevelNum) {
    #[cfg(debug_assertions)]
    set_dev_tracing(_trace_level);
    #[cfg(not(debug_assertions))]
    set_prod_tracing();
}

#[cfg(not(debug_assertions))]
fn set_prod_tracing() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::ERROR)
        .with_file(false)
        .without_time()
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

#[cfg(debug_assertions)]
fn set_dev_tracing(trace_level: &LogLevelNum) {
    use tracing::Level;
    let log_level: Level = match trace_level {
        LogLevelNum::Info => Level::INFO,
        LogLevelNum::Debug => Level::DEBUG,
        LogLevelNum::Trace => Level::TRACE,
        LogLevelNum::Off => Level::ERROR,
        _ => Level::ERROR,
    };
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_writer(std::io::stderr)
        .with_max_level(log_level)
        .with_line_number(true)
        .with_thread_names(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
