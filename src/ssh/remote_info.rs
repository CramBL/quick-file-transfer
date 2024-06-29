use anyhow::{bail, Context};

use crate::{
    config::transfer::send::ssh::{SendSshArgs, TargetComponents},
    ssh::mdns_util,
};

#[cfg(feature = "mdns")]
use std::borrow::Cow;

#[derive(Debug, Clone, Copy)]
pub enum Remote<'a> {
    Ip(&'a str),
    #[cfg(feature = "mdns")]
    MdnsHostname(&'a str),
}

impl<'a> Remote<'a> {
    pub fn new(host: &'a str) -> anyhow::Result<Self> {
        if host.parse::<std::net::IpAddr>().is_ok() {
            return Ok(Self::Ip(host));
        }
        #[cfg(feature = "mdns")]
        if mdns_util::is_mdns_hostname(host) {
            return Ok(Self::MdnsHostname(host));
        }
        bail!("'{host}' is not an IP or a mDNS/DNS-SD hostname");
    }

    #[cfg(feature = "mdns")]
    pub fn to_resolved_ip_str(self, timeout_ms: u64) -> anyhow::Result<Cow<'a, str>> {
        match self {
            Remote::Ip(ip) => Ok(Cow::Borrowed(ip)),
            Remote::MdnsHostname(hn) => {
                let ip = mdns_util::get_remote_ip_from_hostname(
                    hn,
                    timeout_ms,
                    crate::config::misc::IpVersion::V4,
                )?;
                let ip_str = ip.to_string().into();
                Ok(ip_str)
            }
        }
    }

    #[cfg(not(feature = "mdns"))]
    pub fn to_ip_str(self) -> Cow<'a, str> {
        debug_assert!(matches!(self, Remote::Ip(_)));
        match self {
            Remote::Ip(ip) => Cow::Borrowed(ip),
        }
    }
}

pub struct RemoteInfo<'a> {
    pub user: &'a str,
    pub ssh_port: u16,
    pub resolved_ip: Cow<'a, str>,
    pub destination: Cow<'a, str>,
}

impl<'a> RemoteInfo<'a> {
    pub fn new(
        user: &'a str,
        ssh_port: u16,
        resolved_ip: Cow<'a, str>,
        destination: Cow<'a, str>,
    ) -> Self {
        Self {
            user,
            ssh_port,
            resolved_ip,
            destination,
        }
    }

    // Helper to extract the destination from arguments
    fn remote_destination_from_args(ssh_args: &'a SendSshArgs) -> Cow<'a, str> {
        debug_assert!(
            (ssh_args.target.is_some() && ssh_args.destination.is_none())
                || (ssh_args.destination.is_some() && ssh_args.target.is_none())
        );
        let dest_path = if let Some(TargetComponents {
            ref destination, ..
        }) = ssh_args.target
        {
            destination
        } else if let Some(destination) = &ssh_args.destination {
            destination
        } else {
            unreachable!()
        };
        dest_path.to_string_lossy()
    }

    fn remote_user_from_args(ssh_args: &'a SendSshArgs) -> &'a str {
        if let Some(TargetComponents { ref user, .. }) = ssh_args.target {
            user
        } else if let Some(ref user) = ssh_args.user {
            user
        } else {
            unreachable!()
        }
    }

    fn remote_from_args(ssh_args: &'a SendSshArgs) -> Remote {
        if let Some(TargetComponents { ref host, .. }) = ssh_args.target {
            return Remote::new(host)
                .with_context(|| format!("Failed to resolve IP for hostname {host}"))
                .unwrap();
        }

        #[cfg(feature = "mdns")]
        if let Some(ref h) = ssh_args.hostname {
            return Remote::new(h)
                .with_context(|| format!("Failed to resolve IP for hostname {h}"))
                .unwrap();
        }
        if let Some(ref ip) = ssh_args.ip {
            Remote::Ip(ip)
        } else {
            unreachable!()
        }
    }

    pub fn from_args(ssh_args: &'a SendSshArgs) -> Self {
        let user: &str = Self::remote_user_from_args(ssh_args);
        let remote_destination = Self::remote_destination_from_args(ssh_args);
        let remote: Remote = Self::remote_from_args(ssh_args);

        #[cfg(feature = "mdns")]
        let resolved_ip = remote
            .to_resolved_ip_str(ssh_args.mdns_resolve_timeout_ms)
            .expect("Failed to resolve IP for the specified hostname");
        #[cfg(not(feature = "mdns"))]
        let resolved_ip = remote.to_ip_str();

        Self::new(user, ssh_args.ssh_port, resolved_ip, remote_destination)
    }
}
