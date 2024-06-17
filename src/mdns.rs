use crate::config::{MdnsCommand, MdnsDiscoverArgs, MdnsRegisterArgs, MdnsResolveArgs};
use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::{
    collections::HashSet,
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub fn handle_mdns_command(cmd: MdnsCommand) -> Result<()> {
    match cmd {
        MdnsCommand::Discover(MdnsDiscoverArgs {
            service_label,
            service_protocol,
        }) => resolve_mdns(service_label, service_protocol),
        MdnsCommand::Resolve(MdnsResolveArgs {
            hostname,
            timeout_ms,
        }) => resolve_hostname(&hostname, timeout_ms),
        MdnsCommand::Register(MdnsRegisterArgs {
            hostname,
            service_label,
            service_protocol,
            instance_name,
            keep_alice_ms,
            ip,
            port,
        }) => start_mdns_service(
            &hostname,
            &service_label,
            &service_protocol,
            &instance_name,
            keep_alice_ms,
            ip,
            port,
        ),
    }
}

pub fn resolve_mdns(service_label: String, service_protocol: String) -> Result<()> {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    // Browse for a service type.
    let service_type = format!("_{service_label}._{service_protocol}.local.");
    log::info!("Browsing for {service_type}");
    let receiver = mdns.browse(&service_type).expect("Failed to browse");

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
    mdns.shutdown()?;
    Ok(())
}

pub fn resolve_hostname(hostname: &str, timeout_ms: u64) -> Result<()> {
    let stopflag = Arc::new(AtomicBool::new(false));

    let mdns = ServiceDaemon::new()?;
    let mut hostname = hostname.to_owned();
    hostname.push_str(".local.");
    log::info!("Browsing for {hostname}");
    let receiver = mdns.resolve_hostname(&hostname, Some(timeout_ms))?;

    let stopflag_other = Arc::clone(&stopflag);
    let thread_timeout = timeout_ms + 10;
    // Receive the events
    std::thread::spawn(move || {
        let mut hostname = None;
        let mut ip_set = HashSet::<IpAddr>::new();
        while let Ok(hostname_resolution_event) = receiver.recv() {
            match hostname_resolution_event {
                mdns_sd::HostnameResolutionEvent::SearchStarted(s) => {
                    log::debug!("Search started: {s}")
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
                _ => log::debug!("{hostname_resolution_event:?}"),
            }
        }
        println!("Hostname: {}", hostname.unwrap_or_default());
        let ip_count = ip_set.len();
        for (idx, ip) in ip_set.iter().enumerate() {
            if idx == 0 {
                if ip_count == 1 {
                    println!("IP: {ip}");
                } else {
                    println!("IP(s): {ip}");
                }
            } else {
                println!("       {ip}");
            }
        }
        stopflag_other.store(true, Ordering::Relaxed);
    });

    // Wait for the timeout duration or until stopflag is set
    let start_time = std::time::Instant::now();
    while !stopflag.load(Ordering::Relaxed) {
        if start_time.elapsed() >= Duration::from_millis(thread_timeout) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    mdns.shutdown()?;
    Ok(())
}

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
        ServiceInfo::new(&service_type, instance_name, &hostname, &ip_str, port, None)?;

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
