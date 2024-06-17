use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent};
pub fn resolve_mdns(service_type: String) {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    // Browse for a service type.
    let service_type = &service_type;
    log::info!("Browsing for {service_type}");
    let receiver = mdns.browse(service_type).expect("Failed to browse");

    // Receive the browse events in sync
    let mdns_t = mdns.clone();
    std::thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    log::info!("Resolved a new service: {}", info.get_fullname());
                    log::info!("Hostname: {}", info.get_hostname());
                    let receiver = mdns_t
                        .resolve_hostname(info.get_hostname(), Some(4))
                        .unwrap();
                    while let Ok(hostname_resolution_event) = receiver.recv() {
                        match hostname_resolution_event {
                            mdns_sd::HostnameResolutionEvent::SearchStarted(s) => {
                                log::info!("Search started: {s}")
                            }
                            mdns_sd::HostnameResolutionEvent::AddressesFound(s, ip_set) => {
                                log::info!("Hostname found! {s}: {ip_set:?}")
                            }
                            _ => log::debug!("{hostname_resolution_event:?}"),
                        }
                    }
                }
                other_event => {
                    log::info!("Received other event: {:?}", &other_event);
                }
            }
        }
    });

    // Gracefully shutdown the daemon.
    std::thread::sleep(std::time::Duration::from_secs(10));
    mdns.shutdown().unwrap();
}

pub fn resolve_hostname(hostname: String, timeout_ms: u64, oneshot: bool) -> Result<()> {
    let stopflag = Arc::new(AtomicBool::new(false));

    let mdns = ServiceDaemon::new()?;
    log::info!("Browsing for {hostname}");
    let receiver = mdns.resolve_hostname(&hostname, Some(timeout_ms))?;

    let stopflag_other = stopflag.clone();
    // Receive the events
    std::thread::spawn(move || {
        let oneshot = oneshot;
        while let Ok(hostname_resolution_event) = receiver.recv() {
            match hostname_resolution_event {
                mdns_sd::HostnameResolutionEvent::SearchStarted(s) => {
                    log::info!("Search started: {s}")
                }
                mdns_sd::HostnameResolutionEvent::AddressesFound(s, ip_set) => {
                    log::info!("Hostname found! {s}: {ip_set:?}");
                    if oneshot {
                        stopflag_other.store(true, Ordering::Relaxed);
                        break;
                    }
                }
                _ => log::debug!("{hostname_resolution_event:?}"),
            }
        }
        stopflag_other.store(true, Ordering::Relaxed);
    });

    // Gracefully shutdown the daemon.
    if stopflag.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(timeout_ms + 10));
    }
    mdns.shutdown()?;
    Ok(())
}
