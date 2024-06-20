use anyhow::Result;
use mdns_sd::{DaemonStatus, ServiceDaemon};
use std::{collections::HashSet, net::IpAddr, thread};

use super::util::{try_clean_hostname, MdnsServiceInfo};

pub fn resolve_hostname_print_stdout(
    hostname: &str,
    timeout_ms: u64,
    short_circuit: bool,
) -> Result<()> {
    log::info!("Resolving address for {hostname}");
    if let Some(resolved_info) = resolve_mdns_hostname(
        &try_clean_hostname(hostname.into()),
        timeout_ms,
        short_circuit,
    )? {
        println!("{resolved_info}");
    } else {
        log::error!("Failed resolving {hostname}");
    }

    Ok(())
}

/// Resolve mDNS/DNS-SD hostname to [MdnsServiceInfo] which includes a set of IPs of the given hostname.
///
/// # Arguments
/// - `hostname` the mDNS/DNS-SD hostname to resolve
/// - `timeout_ms` maximum time before exiting the resolution attempt (still prints out results)
/// -
pub fn resolve_mdns_hostname(
    hostname: &str,
    timeout_ms: u64,
    short_circuit: bool,
) -> Result<Option<MdnsServiceInfo>> {
    let hostname = try_clean_hostname(hostname.into());
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.resolve_hostname(&hostname, Some(timeout_ms))?;

    let resolved_info = thread::scope(|s| {
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
                        if short_circuit {
                            match mdns.shutdown() {
                                Ok(re) => match re.recv() {
                                    Ok(resp) => {
                                        log::debug!("Shutdown status: {resp:?}");
                                        debug_assert_eq!(resp, DaemonStatus::Shutdown);
                                        // Drain the channel after the daemon is shut down
                                        while let Ok(more_events) = receiver.recv() {
                                            log::trace!("Draining channel: {more_events:?}");
                                        }
                                    }
                                    Err(e) => log::error!("{e}"),
                                },
                                Err(e) => log::error!("{e}"),
                            }
                            break;
                        }
                    }
                    _ => log::trace!("{hostname_resolution_event:?}"),
                }
            }
            hostname.map(|hn| MdnsServiceInfo::new(hn, None, None, ip_set))
        });
        resolver_receiver
            .join()
            .expect("Failed joining resolver receiver thread")
    });

    Ok(resolved_info)
}
