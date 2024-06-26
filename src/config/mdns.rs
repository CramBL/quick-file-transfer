use discover::MdnsDiscoverArgs;
use register::MdnsRegisterArgs;
use resolve::MdnsResolveArgs;

use crate::config::util::*;

pub mod discover;
pub mod register;
pub mod resolve;

/// Holds the mDNS subcommands
#[derive(Debug, Args, Clone)]
#[command(arg_required_else_help = true)]
pub struct MdnsArgs {
    #[command(subcommand)]
    pub subcmd: MdnsCommand,
}

#[derive(Subcommand, Clone, Debug)]
pub enum MdnsCommand {
    /// Use DNS-SD to discover services and attempt to resolve them via mDNS.
    Discover(MdnsDiscoverArgs),
    /// Resolve mDNS hostname
    Resolve(MdnsResolveArgs),
    /// Register a temporary service (for testing)
    Register(MdnsRegisterArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ServiceTypeArgs {
    /// Service label e.g. `foo` -> `_foo._<service_protocol>.local.`
    #[arg(name("service-label"), short('l'), long, value_name = "SERVICE_LABEL")]
    pub label: String,
    /// Service protocol e.g. `tcp` -> `_<service_label>._tcp.local.`
    #[arg(
        name = "service-protocol",
        long,
        visible_alias("proto"),
        value_name = "PROTOCOL"
    )]
    pub protocol: super::misc::TransportLayerProtocol,
}
