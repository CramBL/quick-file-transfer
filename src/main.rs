// Performance lints
#![warn(variant_size_differences)]
#![warn(
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    clippy::mutex_integer,
    clippy::mem_forget,
    clippy::maybe_infinite_iter
)]

pub mod config;
#[cfg(feature = "evaluate-compression")]
pub mod evaluate_compression;
pub mod get_free_port;
#[cfg(feature = "mdns")]
pub mod mdns;
pub mod mmap_reader;
pub mod send;
pub mod server;
#[cfg(feature = "ssh")]
pub mod ssh;
pub mod util;

pub mod run;
pub const TCP_STREAM_BUFSIZE: usize = 2 * 1024;
pub const BUFFERED_RW_BUFSIZE: usize = 32 * 1024;

fn main() -> anyhow::Result<()> {
    let cfg = config::Config::init()?;

    tracing::trace!("{cfg:?}");

    if let Some(shell) = cfg.completions {
        config::Config::generate_completion_script(shell);
        log::info!("Completions generated for {shell:?}. Exiting...");
        return Ok(());
    }

    run::run(&cfg)
}
