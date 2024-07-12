use crate::config::{
    transfer::send::{SendArgs, SendCommand, SendIpArgs},
    Config,
};
#[cfg(feature = "mdns")]
use crate::{config::transfer::send::mdns::SendMdnsArgs, mdns::resolve::resolve_mdns_hostname};

use anyhow::Result;
use client::run_client;

pub mod client;
pub mod util;

pub fn handle_send_cmd(send_args: &SendArgs, _cfg: &Config) -> Result<()> {
    match send_args.subcmd {
        SendCommand::Ip(SendIpArgs {
            ref ip,
            port,
            compression,
        }) => run_client(
            ip.parse()?,
            port,
            send_args.mmap,
            send_args.file.as_slice(),
            send_args.prealloc(),
            compression,
            send_args.tcp_connect_mode(),
        )?,
        #[cfg(feature = "mdns")]
        SendCommand::Mdns(SendMdnsArgs {
            ref hostname,
            timeout_ms,
            ip_version,
            port,
            compression,
        }) => {
            if let Some(resolved_info) = resolve_mdns_hostname(hostname, timeout_ms, true)? {
                if let Some(ip) = resolved_info.get_ip(ip_version) {
                    run_client(
                        *ip,
                        port,
                        send_args.mmap,
                        send_args.file.as_slice(),
                        send_args.prealloc(),
                        compression,
                        send_args.tcp_connect_mode(),
                    )?;
                }
            }
        }
    }
    Ok(())
}
