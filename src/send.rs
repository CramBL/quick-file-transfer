use crate::{
    config::{Config, SendArgs, SendCommand, SendIpArgs, SendMdnsArgs},
    mdns::resolve::resolve_mdns_hostname,
};
use anyhow::Result;
use client::run_client;

mod client;
pub mod util;

pub fn handle_send_cmd(cmd: &SendArgs, _cfg: &Config) -> Result<()> {
    match cmd.subcmd {
        SendCommand::Ip(SendIpArgs {
            ref ip,
            port,
            ref message,
            ref content_transfer_args,
            mmap,
        }) => run_client(
            ip.parse()?,
            port,
            message.as_deref(),
            mmap,
            content_transfer_args,
        )?,
        SendCommand::Mdns(SendMdnsArgs {
            ref hostname,
            timeout_ms,
            ip_version,
            port,
            ref message,
            ref content_transfer_args,
            mmap,
        }) => {
            if let Some(resolved_info) = resolve_mdns_hostname(hostname, timeout_ms, true)? {
                if let Some(ip) = resolved_info.get_ip(ip_version) {
                    run_client(*ip, port, message.as_deref(), mmap, content_transfer_args)?;
                }
            }
        }
    }
    Ok(())
}
