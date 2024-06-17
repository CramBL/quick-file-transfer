use std::{collections::HashSet, fmt, net::IpAddr};

use mdns_sd::ServiceInfo;

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
