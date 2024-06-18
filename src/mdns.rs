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
use util::MdnsServiceInfo;

mod util;

pub fn handle_mdns_command(cmd: MdnsCommand) -> Result<()> {
    match cmd {
        MdnsCommand::Discover(MdnsDiscoverArgs {
            timeout_ms,
            service_type,
        }) => discover_service_type(&service_type.label, &service_type.protocol, timeout_ms),
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
        }) => start_mdns_service(
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

pub fn resolve_hostname_print_stdout(hostname: &str, timeout_ms: u64) -> Result<()> {
    let mut hostname = hostname.to_owned();
    hostname.push_str(".local.");
    log::info!("Resolving address for {hostname}");
    if let Some(resolved_info) = resolve_mdns_hostname(&hostname, timeout_ms)? {
        println!("{resolved_info}");
    } else {
        log::error!("Failed resolving {hostname}");
    }
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

pub fn resolve_mdns_hostname(hostname: &str, timeout_ms: u64) -> Result<Option<MdnsServiceInfo>> {
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
