use anyhow::bail;

use crate::config::ssh::{SendSshArgs, TargetComponents};

use std::{
    net::{IpAddr, ToSocketAddrs},
    path::Path,
};

#[derive(Debug, Clone, Copy)]
pub enum Remote<'a> {
    Ip(&'a str),
    DnsHostname(IpAddr),
    #[cfg(feature = "mdns")]
    MdnsHostname(&'a str),
}

impl<'a> Remote<'a> {
    pub fn new(host: &'a str) -> anyhow::Result<Self> {
        tracing::trace!("Resolving remote: '{host}'");
        if host.parse::<std::net::IpAddr>().is_ok() {
            return Ok(Self::Ip(host));
        }
        #[cfg(feature = "mdns")]
        if crate::ssh::mdns_util::is_mdns_hostname(host) {
            return Ok(Self::MdnsHostname(host));
        }

        let addrs_iter = match (host, 0).to_socket_addrs() {
            Ok(addrs_iter) => addrs_iter,
            Err(e) => {
                #[cfg(feature = "mdns")]
                bail!("'{host}' was not recognized as an IP or a mDNS/DNS-SD hostname, attempt at resolving as a regular DNS hostname failed: {e}");
                #[cfg(not(feature = "mdns"))]
                bail!("'{host}' was not recognized as an IP, (install with mdns feature to resolve mDNS), attempt at resolving as a regular DNS hostname failed: {e}");
            }
        };

        let mut ipv6_fallback: Option<IpAddr> = None;
        for addr in addrs_iter {
            if addr.ip().is_ipv4() {
                return Ok(Self::DnsHostname(addr.ip()));
            } else if ipv6_fallback.is_none() {
                ipv6_fallback = Some(addr.ip());
            }
        }
        if let Some(ipv6) = ipv6_fallback {
            return Ok(Self::DnsHostname(ipv6));
        }
        unreachable!("Should be")
    }

    pub fn to_resolved_ip(
        self,
        #[cfg(feature = "mdns")] timeout_ms: u64,
    ) -> anyhow::Result<IpAddr> {
        match self {
            Remote::Ip(ip) => Ok(ip.parse()?),
            Remote::DnsHostname(hn) => Ok(hn),
            #[cfg(feature = "mdns")]
            Remote::MdnsHostname(hn) => {
                let ip = super::mdns_util::get_remote_ip_from_mdns_hostname(
                    hn,
                    timeout_ms,
                    crate::config::misc::IpVersion::V4,
                )?;
                Ok(ip)
            }
        }
    }
}

pub struct RemoteInfo<'a> {
    pub user: &'a str,
    pub ssh_port: u16,
    pub resolved_ip: IpAddr,
    pub destination: &'a Path,
}

impl<'a> RemoteInfo<'a> {
    pub fn new(user: &'a str, ssh_port: u16, resolved_ip: IpAddr, destination: &'a Path) -> Self {
        Self {
            user,
            ssh_port,
            resolved_ip,
            destination,
        }
    }

    pub fn from_args(ssh_args: &'a SendSshArgs, components: &'a TargetComponents) -> Self {
        let TargetComponents {
            ref user,
            ref host,
            ref destination,
        } = components;

        let resolved_ip: IpAddr = Remote::new(host.as_str())
            .unwrap()
            .to_resolved_ip(
                #[cfg(feature = "mdns")]
                ssh_args.mdns_resolve_timeout_ms,
            )
            .unwrap();

        Self::new(user, ssh_args.ssh_port, resolved_ip, destination)
    }

    pub fn ip(&self) -> IpAddr {
        self.resolved_ip
    }
    pub fn user(&self) -> &str {
        self.user
    }
    pub fn dest(&self) -> &Path {
        self.destination
    }
    pub fn ssh_port(&self) -> u16 {
        self.ssh_port
    }
}
