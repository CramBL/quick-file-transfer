use crate::util::*;
mod util;

pub const IP: &str = "127.0.0.1";
pub const PORT: &str = "1234";

#[test]
pub fn test_get_version() -> TestResult {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("--version");

    let StdoutStderr { stdout, stderr: _ } = process_output_to_stdio(cmd.output()?)?;

    pretty_assert_str_eq!(
        stdout,
        format!("Quick File Transfer {}\n", env!("CARGO_PKG_VERSION"))
    );

    Ok(())
}

#[test]
pub fn test_file_transfer_no_compression_simple() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let client_thread = spawn_client_thread(
        file_to_transfer.path(),
        false,
        ["send", "ip", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["listen", "--ip", IP, "--port", port.as_str(), "-vv"],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr: _,
        },
        ClientOutput {
            client_stdout,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    eprintln!("Client:\nstdout:\n{client_stdout}\nstderr:\n{client_stderr}");

    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_stdout_transfer_no_compression_simple() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");

    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();

    let client_thread = spawn_client_thread(
        file_to_transfer.path(),
        false,
        ["send", "ip", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread =
        spawn_server_thread(None, ["listen", "--ip", IP, "--port", port.as_str(), "-vv"]);

    let (
        ServerOutput {
            server_stdout,
            server_stderr: _,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr: _,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    pretty_assert_str_eq!(TRANSFERED_CONTENTS, server_stdout);

    Ok(())
}

#[test]
pub fn test_stdout_transfer_no_compression_mmap() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();

    let client_thread = spawn_client_thread(
        file_to_transfer.path(),
        false,
        ["send", "ip", "--mmap", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread =
        spawn_server_thread(None, ["listen", "--ip", IP, "--port", port.as_str(), "-vv"]);

    let (
        ServerOutput {
            server_stdout,
            server_stderr: _,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr: _,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    pretty_assert_str_eq!(TRANSFERED_CONTENTS, server_stdout);

    Ok(())
}

#[test]
pub fn test_stdin_stdout_transfer_no_compression() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();

    let client_thread = spawn_client_thread(
        file_to_transfer.path(),
        true,
        ["send", "ip", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread =
        spawn_server_thread(None, ["listen", "--ip", IP, "--port", port.as_str(), "-vv"]);

    let (
        ServerOutput {
            server_stdout,
            server_stderr: _,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr: _,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    pretty_assert_str_eq!(TRANSFERED_CONTENTS, server_stdout);

    Ok(())
}

#[test]
pub fn test_file_transfer_no_compression_with_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let client_thread = spawn_client_thread(
        file_to_transfer.path(),
        false,
        [
            "send",
            "ip",
            "--prealloc",
            IP,
            "--port",
            port.as_str(),
            "-vv",
        ],
    );

    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "listen",
            "--prealloc",
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
        ],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr: _,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr: _,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}
