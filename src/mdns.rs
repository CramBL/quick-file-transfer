use anyhow::Result;

use crate::config::mdns::{
    discover::MdnsDiscoverArgs, register::MdnsRegisterArgs, resolve::MdnsResolveArgs, MdnsCommand,
};

pub mod resolve;

mod discover;
mod register;
mod util;

pub fn handle_mdns_command(cmd: MdnsCommand) -> Result<()> {
    match cmd {
        MdnsCommand::Discover(MdnsDiscoverArgs {
            timeout_ms,
            service_type,
        }) => {
            discover::discover_service_type(&service_type.label, &service_type.protocol, timeout_ms)
        }
        MdnsCommand::Resolve(MdnsResolveArgs {
            hostname,
            timeout_ms,
            short_circuit,
        }) => resolve::resolve_hostname_print_stdout(&hostname, timeout_ms, short_circuit),
        MdnsCommand::Register(MdnsRegisterArgs {
            hostname,
            instance_name,
            keep_alive_ms,
            ip,
            port,
            service_type,
        }) => register::start_mdns_service(
            &hostname,
            &service_type.label,
            &service_type.protocol,
            &instance_name,
            keep_alive_ms,
            ip,
            port,
        ),
    }
}
