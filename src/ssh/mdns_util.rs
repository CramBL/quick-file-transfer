use anyhow::{bail, Result};
use std::net::IpAddr;

use crate::config::misc::IpVersion;

pub fn get_remote_ip_from_mdns_hostname(
    hostname: &str,
    timeout_ms: u64,
    ip_version: IpVersion,
) -> Result<IpAddr> {
    if let Some(info) = crate::mdns::resolve::resolve_mdns_hostname(hostname, timeout_ms, true)? {
        if let Some(ip) = info.get_ip(ip_version) {
            return Ok(ip.to_owned());
        } else {
            bail!("Failed resolving IP for {hostname}")
        }
    }
    bail!("Failed resolving IP for {hostname}")
}

/// Checks if a string ends with either `.local.` or `.local` in which case it is an mDNS hostname
pub fn is_mdns_hostname(hostname: &str) -> bool {
    let suffix_variant1 = ".local.";
    let suffix_variant2 = ".local";
    let hostname_len = hostname.len();

    // Check for the longer suffix first
    if hostname_len >= suffix_variant1.len()
        && hostname
            .chars()
            .rev()
            .zip(suffix_variant1.chars().rev())
            .all(|(h, s)| h == s)
    {
        return true;
    }

    // Check for the shorter suffix if the longer one doesn't match
    if hostname_len >= suffix_variant2.len()
        && hostname
            .chars()
            .rev()
            .zip(suffix_variant2.chars().rev())
            .all(|(h, s)| h == s)
    {
        return true;
    }

    false
}
