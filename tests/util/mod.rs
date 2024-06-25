#![allow(dead_code, unused_imports)]

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
    std::{
        error::Error, fmt::Debug, fmt::Display, fs, io, io::Write, net::IpAddr, path::Path,
        path::PathBuf, process::Output, thread::JoinHandle, time::Duration,
    },
    testresult::{TestError, TestResult},
};

use anyhow::Context;

pub mod test_content;
pub use test_content::*;
mod thread_safe_port_distributor;
pub use thread_safe_port_distributor::{get_free_port, PortGuard};

pub mod regex_util;
pub use regex_util::*;

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
    let StdoutStderr { stdout, stderr } =
        join_thread_and_get_output(client_handle.0).expect("Client thread failed");
    let client_output = ClientOutput {
        client_stdout: stdout,
        client_stderr: stderr,
    };
    let StdoutStderr { stdout, stderr } =
        join_thread_and_get_output(server_thread.0).expect("Server thread failed");
    let server_output = ServerOutput {
        server_stdout: stdout,
        server_stderr: stderr,
    };
    Ok((server_output, client_output))
}

/// Convenience to return stdout/stderr without risking switching them (if instead a tuple of two Strings were used)
#[derive(Debug)]
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
        None,
        args,
        Some(Duration::from_millis(200)),
        stdin_pipe_file,
        false,
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
    spawn_thread_qft_file_transfer("qft server", None, receive_file, args, None, false, true)
}

/// Generic spawn a thread to execute the binary in the server/client file-transfer mode.
/// optionally have the thread sleep before executing the command
pub fn spawn_thread_qft_file_transfer<I, S>(
    thread_name: &str,
    input_file: Option<&Path>,
    output_file: Option<&Path>,
    args: I,
    sleep: Option<Duration>,
    stdin_pipe_file: bool,
    is_server: bool,
) -> Result<JoinHandle<Result<Output>>>
where
    I: IntoIterator<Item = S> + Send + 'static + Debug,
    S: ToOwned + AsRef<std::ffi::OsStr>,
{
    let sender_thread = std::thread::Builder::new().name(thread_name.to_string());
    let handle = sender_thread
        .spawn({
            let input_fpath: Option<String> = input_file.map(|f| f.to_str().unwrap().to_owned());
            let output_fpath: Option<String> = output_file.map(|f| f.to_str().unwrap().to_owned());
            move || {
                let mut cmd = Command::cargo_bin(BIN_NAME)?;
                // Ugly hack to make sure the --file arg is in the right position. CLI needs a refactor after SSH utils are somewhat done.
                if is_server {
                    cmd.arg("listen");
                } else {
                    cmd.arg("send");
                }
                cmd.args(args);
                if let Some(in_fpath) = input_fpath {
                    if stdin_pipe_file {
                        cmd.pipe_stdin(PathBuf::from(&in_fpath))?;
                    } else {
                        cmd.args(["--file", &in_fpath]);
                    }
                }
                if let Some(out_fpath) = output_fpath {
                    cmd.args(["--output", &out_fpath]);
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
