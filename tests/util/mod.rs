#![allow(dead_code, unused_imports)]

use std::{fmt::Debug, net::IpAddr, path::PathBuf, thread::JoinHandle, time::Duration};
/// Re-export some common utilities for system tests
pub use {
    anyhow::Result,
    assert_cmd::{prelude::*, Command},
    assert_fs::{fixture::ChildPath, prelude::*, TempDir},
    predicates::prelude::*,
    pretty_assertions::{
        assert_eq as pretty_assert_eq, assert_ne as pretty_assert_ne,
        assert_str_eq as pretty_assert_str_eq,
    },
    std::{error::Error, fmt::Display, fs, io, io::Write, path::Path, process::Output},
    testresult::{TestError, TestResult},
};

pub const BIN_NAME: &str = "qft";

pub struct ClientOutput {
    pub client_stdout: String,
    pub client_stderr: String,
}

pub struct ServerOutput {
    pub server_stdout: String,
    pub server_stderr: String,
}

pub type ThreadOutputHandle = JoinHandle<Result<Output>>;
// Newtypes around ThreadOutputHandle
pub struct ServerHandle(pub ThreadOutputHandle);
pub struct ClientHandle(pub ThreadOutputHandle);

/// Join server and client threads and return their respective stdout and stderr as Strings.
pub fn join_server_and_client_get_outputs(
    server_thread: ServerHandle,
    client_handle: ClientHandle,
) -> Result<(ServerOutput, ClientOutput)> {
    let StdoutStderr { stdout, stderr } = join_thread_and_get_output(client_handle.0)?;
    let client_output = ClientOutput {
        client_stdout: stdout,
        client_stderr: stderr,
    };
    let StdoutStderr { stdout, stderr } = join_thread_and_get_output(server_thread.0)?;
    let server_output = ServerOutput {
        server_stdout: stdout,
        server_stderr: stderr,
    };
    Ok((server_output, client_output))
}

/// Convenience to return stdout/stderr without risking switching them (if instead a tuple of two Strings were used)
pub struct StdoutStderr {
    pub stdout: String,
    pub stderr: String,
}

/// Converts process output to their `status`, `stdout`, and `stderr` components,
/// asserts the output status is success (and prints diagnostics if it isn't), and finally
/// returns `stdout` & `stderr` as Strings for convenience, wrapped in a `StdoutStderr` instance for type-safety.
pub fn process_output_to_stdio(output: Output) -> Result<StdoutStderr> {
    let Output {
        status,
        stdout,
        stderr,
    } = output;

    let stdout = String::from_utf8(stdout)?;
    let stderr = String::from_utf8(stderr)?;

    assert!(
        status.success(),
        "Command failed with status: {status}\n - stdout: {stdout}\n - stderr: {stderr}"
    );

    Ok(StdoutStderr { stdout, stderr })
}

/// Join a thread that returns process output (status, stdout, stderr) and convert stdout and stderr to
/// String (for convenience) and assert that the process (thread) exited successfully, otherwise print process output
/// finally return the convenience (and type-safe) `StdoutStderr` with the threads stdout & stderr contents.
pub fn join_thread_and_get_output(
    thread_handle: JoinHandle<Result<Output>>,
) -> Result<StdoutStderr> {
    let output = thread_handle.join().expect("Failed joining thread")?;
    process_output_to_stdio(output)
}

/// Spawn the client thread that transfers content to the server
///
/// If `stdin_pipe_file` is `true`, the file contents will be piped into the process via stdin
/// instead of passing the path of the file to the binary
pub fn spawn_client_thread<I, S>(
    file_for_transfer: &Path,
    stdin_pipe_file: bool,
    args: I,
) -> Result<JoinHandle<Result<Output>>>
where
    I: IntoIterator<Item = S> + Send + 'static + Debug,
    S: ToOwned + AsRef<std::ffi::OsStr>,
{
    spawn_thread_qft_file_transfer(
        "qft client",
        Some(file_for_transfer),
        args,
        Some(Duration::from_millis(200)),
        stdin_pipe_file,
    )
}

/// Spawn the server thread that receives content from the client
/// If no receive_file is specified, prints contents to stdout
pub fn spawn_server_thread<I, S>(
    receive_file: Option<&Path>,
    args: I,
) -> Result<JoinHandle<Result<Output>>>
where
    I: IntoIterator<Item = S> + Send + 'static + Debug,
    S: ToOwned + AsRef<std::ffi::OsStr>,
{
    spawn_thread_qft_file_transfer("qft server", receive_file, args, None, false)
}

/// Generic spawn a thread to execute the binary in the server/client file-transfer mode.
/// optionally have the thread sleep before executing the command
pub fn spawn_thread_qft_file_transfer<I, S>(
    thread_name: &str,
    file: Option<&Path>,
    args: I,
    sleep: Option<Duration>,
    stdin_pipe_file: bool,
) -> Result<JoinHandle<Result<Output>>>
where
    I: IntoIterator<Item = S> + Send + 'static + Debug,
    S: ToOwned + AsRef<std::ffi::OsStr>,
{
    let sender_thread = std::thread::Builder::new().name(thread_name.to_string());
    let handle = sender_thread
        .spawn({
            let fpath: Option<String> = file.map(|f| f.to_str().unwrap().to_owned());
            move || {
                let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
                cmd.args(args);
                if let Some(fpath) = fpath {
                    if stdin_pipe_file {
                        cmd.pipe_stdin(PathBuf::from(&fpath))?;
                    } else {
                        cmd.args(["--file", &fpath]);
                    }
                }
                cmd.timeout(Duration::from_secs(5));

                eprintln!("Command: {cmd:?}");
                if let Some(sleep_duration) = sleep {
                    std::thread::sleep(sleep_duration);
                }

                let out = cmd.output()?;
                Ok(out)
            }
        })
        .context(format!("Failed spawning {thread_name} thread"));

    handle
}

/// Generic spawn a thread to execute the binary with the given args.
/// Optionally have the thread sleep before executing the command
pub fn spawn_thread_qft<I, S>(
    thread_name: &str,
    args: I,
    sleep: Option<Duration>,
) -> Result<JoinHandle<Result<Output>>>
where
    I: IntoIterator<Item = S> + Send + 'static,
    S: ToOwned + AsRef<std::ffi::OsStr>,
{
    let sender_thread = std::thread::Builder::new().name(thread_name.to_string());
    let handle = sender_thread
        .spawn({
            move || {
                let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
                cmd.args(args);
                if let Some(sleep_duration) = sleep {
                    std::thread::sleep(sleep_duration);
                }

                let out = cmd.output()?;
                Ok(out)
            }
        })
        .context(format!("Failed spawning {thread_name} thread"));

    handle
}

use anyhow::Context;
pub use thread_safe_port_distributor::{get_free_port, PortGuard};
/// Implements utility to safely get a free port for a given IP in parallel from a large number of threads.
///
/// This is necessary for running tests in parallel where each test spawns a server/client thread and needs a free port for that purpose.
pub mod thread_safe_port_distributor {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
    use std::sync::{Mutex, OnceLock};

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
        for port in 1024..65535 {
            if is_port_available(ip, port) {
                let mut ports = ports.lock().unwrap();
                if !ports.contains(&port) {
                    ports.insert(port);
                    let port_wrapper = PortGuard {
                        port_num: port,
                        // Leak the port string to get static liftime, the memory will be freed once the test process finishes
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
}

/// matches a single ANSI escape code
pub const ANSI_ESCAPE_REGEX: &str = r"(\x9B|\x1B\[)[0-?]*[ -\/]*[@-~]";
/// WARN prefix with an ANSI escape code
pub const WARN_PREFIX: &str = concat!("WARN ", r"(\x9B|\x1B\[)[0-?]*[ -\/]*[@-~]");
pub const ERROR_PREFIX: &str = concat!("ERROR ", r"(\x9B|\x1B\[)[0-?]*[ -\/]*[@-~]");

/// Helper function to match the raw output of stderr or stdout, with a pattern a fixed amount of times, case insensitive
pub fn match_count<S>(
    case_sensitive: bool,
    haystack: &str,
    re: S,
    expect_match: usize,
) -> TestResult
where
    S: AsRef<str> + ToOwned + Display + Into<String>,
{
    // Build regex pattern
    let regex_pattern = if case_sensitive {
        re.to_string()
    } else {
        format!("(?i){re}")
    };
    let re = fancy_regex::Regex::new(&regex_pattern)?;
    // Count the number of matches
    let match_count = re.find_iter(haystack).count();
    // Assert that the number of matches is equal to the expected number of matches
    pretty_assert_eq!(
        match_count, expect_match,
        "regex: {re} - expected match count: {expect_match}, got {match_count}\nFailed to match on:\n{haystack}"
    );
    Ok(())
}

/// Helper function takes in the output of stderr and asserts that there are no errors, warnings, or thread panics.
pub fn assert_no_errors_or_warn(stderr: &str) -> TestResult {
    match_count(true, stderr, ERROR_PREFIX, 0)?;
    match_count(true, stderr, WARN_PREFIX, 0)?;
    match_count(false, stderr, "thread.*panicked", 0)?;
    Ok(())
}
