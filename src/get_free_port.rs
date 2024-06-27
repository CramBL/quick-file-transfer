use anyhow::bail;

use crate::{
    config::get_free_port::GetFreePortArgs,
    util::{self, get_free_port_in_range},
};

pub fn handle_get_free_port(args: &GetFreePortArgs) -> anyhow::Result<()> {
    if args.start_port.is_none() {
        log::debug!("Retrieving any free port for IP: {}", args.ip);
    }

    if let Some(start_port_range) = args.start_port {
        let end_port = args.end_port.unwrap_or(u16::MAX);
        log::debug!(
            "Retrieving free port for IP: {ip}, in range: {start_port_range}:{end_port}",
            ip = args.ip
        );
        if let Some(port) = get_free_port_in_range(&args.ip, start_port_range, end_port) {
            println!("{port}");
        } else {
            bail!("Could not retrieve free port");
        }
    } else if let Some(port) = util::get_free_port(&args.ip) {
        println!("{port}");
    } else {
        bail!("Could not retrieve free port");
    }
    Ok(())
}
