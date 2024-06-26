use super::BIN_NAME;
use anyhow::{Context, Result};
use assert_cmd::{cargo::CommandCargoExt, Command};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    process::Output,
    thread::JoinHandle,
    time::Duration,
};

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
                let mut cmd = Command::cargo_bin(BIN_NAME)?;
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

/// Generic spawn a thread to execute the binary with the given args.
/// Optionally have the thread sleep before executing the command
pub fn spawn_cmd_thread(
    thread_name: &str,
    mut cmd: Command,
    sleep: Option<Duration>,
) -> Result<JoinHandle<Result<Output>>> {
    let sender_thread = std::thread::Builder::new().name(thread_name.to_string());
    sender_thread
        .spawn({
            move || {
                if let Some(sleep_duration) = sleep {
                    std::thread::sleep(sleep_duration);
                }

                let out = cmd.output()?;
                Ok(out)
            }
        })
        .context(format!("Failed spawning {thread_name} thread"))
}
