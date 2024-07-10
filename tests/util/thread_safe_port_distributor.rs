/// Implements utility to safely get a free port for a given IP in parallel from a large number of threads.
///
/// This is necessary for running tests in parallel where each test spawns a server/client thread and needs a free port for that purpose.
use std::cell::RefCell;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::{Mutex, OnceLock};

use rand::Rng;

// Stores taken ports
static PORTS: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();

/// Wraps a free port to guarantee the port is released/freed on drop
pub struct PortGuard {
    port_num: u16,
    port_str: &'static str,
}

impl PortGuard {
    pub fn as_str(&self) -> &'static str {
        self.port_str
    }
}

impl Drop for PortGuard {
    fn drop(&mut self) {
        release_port(self.port_num);
    }
}

/// Get a free port from an IP, e.g. `127.0.0.1`
///
/// # Returns
/// An `Option<PortGuard>` where the `PortGuard` frees the port on drop.
///
/// # Example
/// ```ignore
/// let port = get_free_port("127.0.0.1").unwrap();
/// println!("{}", port.as_str()); // "8080" (for example)
/// ```
pub fn get_free_port(ip: &str) -> Option<PortGuard> {
    let ip: Ipv4Addr = ip
        .parse()
        .map_err(|e| format!("Invalid IP address: {e}"))
        .unwrap();
    let ports: &Mutex<HashSet<u16>> = get_ports();

    let mut rng = rand::thread_rng();
    let start_port = rng.gen_range(49152..=61000);
    // Create a wrapping iterator that starts from `start_port` and wraps around until `start_port - 1`
    let port_range = (start_port..=61000).chain(49152..start_port);
    // This range is valid for later windows and should be for most or all unix.
    for port in port_range {
        if is_port_available(ip, port) {
            let mut ports = ports.lock().unwrap();
            if !ports.contains(&port) {
                ports.insert(port);
                let port_wrapper = PortGuard {
                    port_num: port,
                    // Leak the port string to get static lifetime, the memory will be freed once the test process finishes
                    port_str: Box::leak(port.to_string().into_boxed_str()),
                };
                return Some(port_wrapper);
            }
        }
    }
    None
}

fn get_ports() -> &'static Mutex<HashSet<u16>> {
    PORTS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn is_port_available<I: Into<IpAddr>>(ip: I, port: u16) -> bool {
    let addr = SocketAddr::from((ip, port));
    TcpListener::bind(addr).is_ok()
}
fn release_port(port: u16) {
    let ports = get_ports();
    let mut ports = ports.lock().unwrap();
    ports.remove(&port);
}
