use std::{net::IpAddr, thread, time::Duration};

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub fn start_mdns_service(
    hostname: &str,
    service_label: &str,
    service_protocol: &str,
    instance_name: &str,
    keep_alive_ms: u64,
    ip: Option<String>,
    port: u16,
) -> Result<()> {
    let mdns = ServiceDaemon::new()?;

    let service_type = format!("_{service_label}._{service_protocol}.local.");
    let no_ip_provided = ip.is_none();
    let ip_str: String = if let Some(ip) = ip {
        let _ip_addr: IpAddr = ip.parse().expect("Invalid IP address");
        ip
    } else {
        "".to_string()
    };
    let hostname = format!("{hostname}.local.");

    let mut new_service =
        ServiceInfo::new(&service_type, instance_name, &hostname, ip_str, port, None)?;

    if no_ip_provided {
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
    mdns.shutdown()?;
    Ok(())
}
