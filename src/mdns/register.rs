use std::{net::IpAddr, thread, time::Duration};

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::mdns::util::{self, mdns_daemon_shutdown};

pub fn start_mdns_service(
    hostname: &str,
    service_label: &str,
    service_protocol: &str,
    instance_name: &str,
    keep_alive_ms: u64,
    ip: Option<&str>,
    port: u16,
) -> Result<()> {
    let mdns = ServiceDaemon::new()?;

    let service_type = format!("_{service_label}._{service_protocol}.local.");
    let ip_str: Option<String> = if let Some(ip) = ip {
        let _ip_addr: IpAddr = ip.parse().expect("Invalid IP address");
        Some(ip.to_owned())
    } else {
        None
    };
    let hostname = util::try_clean_hostname(hostname.into());

    let mut new_service = ServiceInfo::new(
        &service_type,
        instance_name,
        &hostname,
        ip_str.as_deref().unwrap_or_default(),
        port,
        None,
    )?;

    if ip_str.is_none() {
        new_service = new_service.enable_addr_auto();
    }

    log::info!(
        "Registering:\n\
    \tHostname:  {hostname}\n\
    \tType:      {type_name}\n\
    \tFull Name: {full_name}\n\
    ",
        hostname = new_service.get_hostname(),
        type_name = new_service.get_type(),
        full_name = new_service.get_fullname(),
    );

    // Register with the daemon, which publishes the service.
    mdns.register(new_service)?;

    let keepalive_dur = Duration::from_millis(keep_alive_ms);
    log::info!("Keeping alive for: {keepalive_dur:?}");
    thread::sleep(keepalive_dur);
    mdns_daemon_shutdown(&mdns);
    Ok(())
}
