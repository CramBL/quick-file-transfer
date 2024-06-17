// Performance lints
#![warn(variant_size_differences)]
#![warn(
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    clippy::mutex_integer,
    clippy::mem_forget,
    clippy::maybe_infinite_iter
)]

use anyhow::Result;
use client::run_client;
use config::{Command, Config, ListenArgs, SendArgs, SendCommand, SendIpArgs, SendMdnsArgs};
use mdns::handle_mdns_command;
use server::run_server;

pub mod client;
pub mod config;
pub mod mdns;
pub mod mmap_reader;
pub mod server;
pub mod util;

pub const TCP_STREAM_BUFSIZE: usize = 2 * 1024;
pub const BUFFERED_RW_BUFSIZE: usize = 32 * 1024;

fn main() -> Result<()> {
    let cfg = Config::init()?;

    log::trace!("{cfg:?}");
    //log::debug!("{:?}", cfg.address());

    match cfg.command.clone() {
        Command::Listen(ListenArgs { ip }) => run_server(&ip, &cfg),
        Command::Send(cmd) => handle_send_cmd(cmd.subcmd, &cfg),
        Command::Mdns(cmd) => handle_mdns_command(cmd.subcmd),
    }
}

pub fn handle_send_cmd(cmd: SendCommand, cfg: &Config) -> Result<()> {
    match cmd {
        SendCommand::Ip(SendIpArgs { ip }) => run_client(&ip, cfg),
        SendCommand::Mdns(SendMdnsArgs { hostname }) => {
            todo!("Resolve hostname to ip");
            //run_client(&ip, cfg)
        }
    }
}
