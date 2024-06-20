use crate::config::{MdnsCommand, MdnsDiscoverArgs, MdnsRegisterArgs, MdnsResolveArgs};
use anyhow::Result;
use mdns_sd::ServiceDaemon;
use std::{
    collections::HashSet,
    net::IpAddr,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};
use util::MdnsServiceInfo;

mod discover;
mod register;
mod util;

pub fn handle_mdns_command(cmd: MdnsCommand) -> Result<()> {
    match cmd {
        MdnsCommand::Discover(MdnsDiscoverArgs {
            timeout_ms,
            service_type,
        }) => {
            discover::discover_service_type(&service_type.label, &service_type.protocol, timeout_ms)
        }
        MdnsCommand::Resolve(MdnsResolveArgs {
            hostname,
            timeout_ms,
        }) => resolve_hostname_print_stdout(&hostname, timeout_ms),
        MdnsCommand::Register(MdnsRegisterArgs {
            hostname,
            instance_name,
            keep_alive_ms,
            ip,
            port,
            service_type,
        }) => register::start_mdns_service(
            &hostname,
            &service_type.label,
            &service_type.protocol,
            &instance_name,
            keep_alive_ms,
            ip,
            port,
        ),
    }
}

pub fn resolve_hostname_print_stdout(hostname: &str, timeout_ms: u64) -> Result<()> {
    let mut hostname = hostname.to_owned();
    if !hostname.ends_with(".local") && !hostname.ends_with(".local.") {
        hostname.push_str(".local.");
    }
    log::info!("Resolving address for {hostname}");
    if let Some(resolved_info) = resolve_mdns_hostname(&hostname, timeout_ms)? {
        println!("{resolved_info}");
    } else {
        log::error!("Failed resolving {hostname}");
    }
    Ok(())
}

pub fn resolve_mdns_hostname(hostname: &str, timeout_ms: u64) -> Result<Option<MdnsServiceInfo>> {
    let mut hostname = hostname;

    // If the supplied hostname does not end in a dot e.g. `foo.local`, try adding a dot and continuing
    // This is simply to fix the 'convenience case' where the ending dot is omitted from the hostname.
    // The dot-ending is a fully qualified path that DNS resolvers typically add if it is not present.
    let dot_corrected_hostname = if hostname.chars().last().unwrap_or_default() != '.' {
        let mut hostname_try_fix = hostname.to_owned();
        hostname_try_fix.push('.');
        hostname_try_fix
    } else {
        String::with_capacity(0)
    };
    if !dot_corrected_hostname.is_empty() {
        hostname = dot_corrected_hostname.as_str()
    }

    let stopflag = AtomicBool::new(false);
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.resolve_hostname(hostname, Some(timeout_ms))?;

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
