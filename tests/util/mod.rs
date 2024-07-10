#![allow(dead_code, unused_imports)]

use std::process::ExitStatus;

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

pub mod thread_cmd_spawning;
use thread_cmd_spawning::*;
pub use thread_cmd_spawning::{
    spawn_client_thread, spawn_cmd_thread, spawn_server_thread, spawn_thread_qft,
};

pub const BIN_NAME: &str = "qft";

pub struct ClientOutput {
    pub client_status: ExitStatus,
    pub client_stdout: String,
    pub client_stderr: String,
}

impl OutputFailPrint for ClientOutput {
    fn status(&self) -> ExitStatus {
        self.client_status
    }

    fn stdout(&self) -> &str {
        &self.client_stdout
    }

    fn stderr(&self) -> &str {
        &self.client_stderr
    }
}

pub struct ServerOutput {
    pub server_status: ExitStatus,
    pub server_stdout: String,
    pub server_stderr: String,
}

impl OutputFailPrint for ServerOutput {
    fn status(&self) -> ExitStatus {
        self.server_status
    }

    fn stdout(&self) -> &str {
        &self.server_stdout
    }

    fn stderr(&self) -> &str {
        &self.server_stderr
    }
}

pub trait OutputFailPrint {
    fn status(&self) -> ExitStatus;
    fn stdout(&self) -> &str;
    fn stderr(&self) -> &str;
    fn failed(&self) -> bool {
        !self.status().success()
    }
    fn success(&self) -> bool {
        self.status().success()
    }
    fn display_diagnostics(&self) {
        println!(
            "Command failed with status: {status}\n - stdout: {stdout}\n - stderr: {stderr}",
            status = self.status(),
            stdout = self.stdout(),
            stderr = self.stderr()
        );
    }
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
    let ProcessOutput {
        status,
        stdout,
        stderr,
    } = join_thread_and_get_output(client_handle.0).expect("Client thread failed");
    let client_output = ClientOutput {
        client_status: status,
        client_stdout: stdout,
        client_stderr: stderr,
    };
    let ProcessOutput {
        status,
        stdout,
        stderr,
    } = join_thread_and_get_output(server_thread.0).expect("Server thread failed");
    let server_output = ServerOutput {
        server_status: status,
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
/// Convenience as above but includes exit status
#[derive(Debug)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Converts process output to their `status`, `stdout`, and `stderr` components,
/// asserts the output status is success (and prints diagnostics if it isn't), and finally
/// returns `stdout` & `stderr` as Strings for convenience, wrapped in a `StdoutStderr` instance for type-safety.
pub fn process_output_to_stdio_if_success(output: Output) -> Result<StdoutStderr> {
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

/// Converts process output to their `status`, `stdout`, and `stderr` components,
/// returns `stdout` & `stderr` as Strings for convenience, wrapped in a `ProcessOutput` instance for type-safety.
pub fn process_output(output: Output) -> Result<ProcessOutput> {
    let Output {
        status,
        stdout,
        stderr,
    } = output;

    let stdout = String::from_utf8(stdout)?;
    let stderr = String::from_utf8(stderr)?;

    Ok(ProcessOutput {
        status,
        stdout,
        stderr,
    })
}

/// Join a thread that returns process output (status, stdout, stderr) and convert stdout and stderr to
/// String (for convenience) and assert that the process (thread) exited successfully, otherwise print process output
/// finally return the convenience (and type-safe) `StdoutStderr` with the threads stdout & stderr contents.
pub fn join_thread_and_get_output_if_success(
    thread_handle: JoinHandle<Result<Output>>,
) -> Result<StdoutStderr> {
    let output = thread_handle.join().expect("Failed joining thread")?;
    process_output_to_stdio_if_success(output)
}

pub fn join_thread_and_get_output(
    thread_handle: JoinHandle<Result<Output>>,
) -> Result<ProcessOutput> {
    let output = thread_handle.join().expect("Failed joining thread")?;
    process_output(output)
}
