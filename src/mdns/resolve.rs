use anyhow::Result;
use mdns_sd::ServiceDaemon;
use std::{
    collections::HashSet,
    net::IpAddr,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use super::util::{try_clean_hostname, MdnsServiceInfo};

pub fn resolve_hostname_print_stdout(hostname: &str, timeout_ms: u64) -> Result<()> {
    let hostname = try_clean_hostname(hostname.into());
    log::info!("Resolving address for {hostname}");
    if let Some(resolved_info) = resolve_mdns_hostname(&hostname, timeout_ms)? {
        println!("{resolved_info}");
    } else {
        log::error!("Failed resolving {hostname}");
    }
    Ok(())
}

pub fn resolve_mdns_hostname(hostname: &str, timeout_ms: u64) -> Result<Option<MdnsServiceInfo>> {
    let hostname = try_clean_hostname(hostname.into());
    let stopflag = AtomicBool::new(false);
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.resolve_hostname(&hostname, Some(timeout_ms))?;

    let resolved_info = std::thread::scope(|s| {
        let resolver_receiver = s.spawn(|| {
            let mut hostname = None;
            let mut ip_set = HashSet::<IpAddr>::new();
            while let Ok(hostname_resolution_event) = receiver.recv() {
                match hostname_resolution_event {
                    mdns_sd::HostnameResolutionEvent::SearchStarted(s) => {
                        log::trace!("Search started: {s}")
                    }
                    mdns_sd::HostnameResolutionEvent::AddressesFound(s, recv_ip_set) => {
                        log::debug!("Hostname found! {s}: {recv_ip_set:?}");
                        if let Some(h) = hostname.as_deref() {
                            debug_assert_eq!(h, s);
                        } else {
                            hostname = Some(s);
                        }
                        ip_set.extend(recv_ip_set);
                    }
                    _ => log::trace!("{hostname_resolution_event:?}"),
                }
            }
            stopflag.store(true, Ordering::Relaxed);
            hostname.map(|hn| MdnsServiceInfo::new(hn, None, None, ip_set))
        });
        let _resolver_watchdog = s.spawn(|| {
            // Wait for the timeout duration or until stopflag is set
            let start_time = std::time::Instant::now();
            while !stopflag.load(Ordering::Relaxed) {
                if start_time.elapsed() >= Duration::from_millis(timeout_ms) {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        resolver_receiver
            .join()
            .expect("Failed joining resolver receiver thread")
    });

    Ok(resolved_info)
}
