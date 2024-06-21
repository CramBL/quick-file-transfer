use std::{borrow::Cow, collections::HashSet, fmt, net::IpAddr};

use mdns_sd::{DaemonStatus, ServiceDaemon, ServiceInfo};

use crate::config::IpVersion;

#[derive(Debug, PartialEq)]
pub struct MdnsServiceInfo {
    hostname: String,
    type_name: Option<String>,
    full_name: Option<String>,
    ips: HashSet<IpAddr>,
}

impl MdnsServiceInfo {
    pub fn new(
        hostname: String,
        typename: Option<String>,
        full_name: Option<String>,
        ips: HashSet<IpAddr>,
    ) -> Self {
        Self {
            hostname,
            type_name: typename,
            full_name,
            ips,
        }
    }

    pub fn add_ips(&mut self, ip_set: &HashSet<IpAddr>) {
        self.ips.extend(ip_set);
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn ips(&self) -> &HashSet<IpAddr> {
        &self.ips
    }

    pub fn any_ipv4(&self) -> Option<&IpAddr> {
        self.ips.iter().find(|a| a.is_ipv4())
    }

    pub fn any_ipv6(&self) -> Option<&IpAddr> {
        self.ips.iter().find(|a| a.is_ipv6())
    }

    pub fn get_ip(&self, preferred_version: IpVersion) -> Option<&IpAddr> {
        match preferred_version {
            IpVersion::V4 => self.any_ipv4().or(self.any_ipv6()),
            IpVersion::V6 => self.any_ipv6().or(self.any_ipv4()),
        }
    }
}

impl fmt::Display for MdnsServiceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Hostname:  {}", self.hostname)?;

        if let Some(tn) = self.type_name.as_deref() {
            writeln!(f, "Type Name: {tn}")?;
        }
        if let Some(fullname) = self.full_name.as_deref() {
            writeln!(f, "Full Name: {fullname}")?;
        }

        let ip_count = self.ips.len();
        for (idx, ip) in self.ips.iter().enumerate() {
            if idx == 0 {
                if ip_count == 1 {
                    writeln!(f, "IP: {ip}")?;
                } else {
                    writeln!(f, "IP(s): {ip}")?;
                }
            } else {
                writeln!(f, "       {ip}")?;
            }
        }
        Ok(())
    }
}

impl From<ServiceInfo> for MdnsServiceInfo {
    fn from(value: ServiceInfo) -> Self {
        Self {
            hostname: value.get_hostname().to_owned(),
            type_name: Some(value.get_type().to_owned()),
            full_name: Some(value.get_fullname().to_owned()),
            ips: value.get_addresses().to_owned(),
        }
    }
}

/// If the supplied hostname does not end in a dot e.g. `foo.local`, try adding a dot and continuing
/// This is simply to fix the 'convenience case' where the ending dot is omitted from the hostname.
/// The dot-ending is a fully qualified path that DNS resolvers typically add if it is not present.
pub fn try_clean_hostname(hostname: Cow<'_, str>) -> Cow<'_, str> {
    let dot_corrected_hostname = if hostname.chars().last().unwrap_or_default() == '.' {
        hostname
    } else {
        let mut hostname_try_fix = hostname.into_owned();
        hostname_try_fix.push('.');
        hostname_try_fix.into()
    };
    fully_qualify_hostname(dot_corrected_hostname)
}

/// Takes a hostname ending with a dot and fully qualifies it with `local.` if it isn't already
///
/// # Note
///
/// Expected to be called after [try_clean_hostname] to ensure it ends with a dot.
fn fully_qualify_hostname(hostname: Cow<'_, str>) -> Cow<'_, str> {
    if hostname.ends_with("local.") {
        hostname
    } else {
        let mut fully_qualified_hostname = hostname.into_owned();
        fully_qualified_hostname.push_str("local.");
        fully_qualified_hostname.into()
    }
}

/// Shutdown the [ServiceDaemon] and receive the [DaemonStatus](DaemonStatus::Shutdown) response while logging relevant steps
pub fn mdns_daemon_shutdown(mdns: &ServiceDaemon) {
    let shutdown_res = mdns.shutdown();
    debug_assert!(shutdown_res.is_ok());
    match shutdown_res {
        Ok(re) => {
            let recv_res = re.recv();
            debug_assert!(recv_res.is_ok());
            match recv_res {
                Ok(resp) => {
                    log::debug!("Shutdown status: {resp:?}");
                    debug_assert_eq!(resp, DaemonStatus::Shutdown);
                }
                Err(e) => log::error!("{e}"),
            }
        }
        Err(e) => log::error!("{e}"),
    }
}
