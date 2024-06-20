use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crate::mdns::util::MdnsServiceInfo;

pub fn discover_service_type(
    service_label: &str,
    service_protocol: &str,
    timeout_ms: u64,
) -> Result<()> {
    let stopflag = Arc::new(AtomicBool::new(false));
    let stopflag_child = Arc::clone(&stopflag);

    let mdns = ServiceDaemon::new()?;

    // Browse for a service type.
    let service_type = format!("_{service_label}._{service_protocol}.local.");
    log::info!("Browsing for {service_type}");

    let receiver = mdns.browse(&service_type)?;

    std::thread::spawn(move || {
        let mut discovered_services: Vec<MdnsServiceInfo> = vec![];
        loop {
            if stopflag_child.load(Ordering::Relaxed) {
                break;
            }
            while let Ok(event) = receiver.try_recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        log::info!("Resolved a new service: {}", info.get_fullname());
                        log::debug!("Hostname: {}", info.get_hostname());
                        log::debug!("IP: {:?}", info.get_addresses());
                        if let Some(service_info) = discovered_services
                            .iter_mut()
                            .find(|s| s.hostname() == info.get_hostname())
                        {
                            service_info.add_ips(info.get_addresses());
                        } else {
                            discovered_services.push(info.into());
                        }
                    }
                    other_event => {
                        log::debug!("Received other event: {:?}", &other_event);
                    }
                }
            }
        }
        log::info!(
            "Discovered {} service{}!",
            discovered_services.len(),
            if discovered_services.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        for discovered_service in discovered_services.iter() {
            println!("{discovered_service}");
        }
        stopflag_child.store(true, Ordering::Relaxed);
    });

    // Wait for the timeout duration or until stopflag is set
    let start_time = std::time::Instant::now();
    while !stopflag.load(Ordering::Relaxed) {
        if start_time.elapsed() >= Duration::from_millis(timeout_ms) {
            stopflag.store(true, Ordering::Relaxed);
            thread::sleep(Duration::from_millis(200));
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    mdns.shutdown()?;
    Ok(())
}
