use crate::util::*;

pub const IP: &str = "127.0.0.1";

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
        ["ip", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["--ip", IP, "--port", port.as_str(), "-vv"],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
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

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
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
        ["ip", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread = spawn_server_thread(None, ["--ip", IP, "--port", port.as_str(), "-vv"]);

    let (
        ServerOutput {
            server_stdout,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
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
        ["ip", "--mmap", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread = spawn_server_thread(None, ["--ip", IP, "--port", port.as_str(), "-vv"]);

    let (
        ServerOutput {
            server_stdout,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;

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
        ["ip", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread = spawn_server_thread(None, ["--ip", IP, "--port", port.as_str(), "-vv"]);

    let (
        ServerOutput {
            server_stdout,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
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
        ["ip", "--prealloc", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["--prealloc", "--ip", IP, "--port", port.as_str(), "-vv"],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_bzip2_default_with_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--prealloc",
        "bzip2",
    ]);
    let client_thread = spawn_cmd_thread("Client thread", cmd, Some(Duration::from_millis(200)));
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "bzip2",
            "--prealloc",
        ],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_gzip_default_with_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--prealloc",
        "gzip",
    ]);
    let client_thread = spawn_cmd_thread("Client thread", cmd, Some(Duration::from_millis(200)));
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "gzip",
            "--prealloc",
        ],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_lz4_default_with_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--prealloc",
        "lz4",
    ]);
    let client_thread = spawn_cmd_thread("Client thread", cmd, Some(Duration::from_millis(200)));
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "lz4",
            "--prealloc",
        ],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_xz_default_with_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--prealloc",
        "xz",
    ]);
    let client_thread = spawn_cmd_thread("Client thread", cmd, Some(Duration::from_millis(200)));
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "xz",
            "--prealloc",
        ],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_bzip2_compr_level_1() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "bzip2",
        "1",
    ]);
    let client_thread = spawn_cmd_thread("Client thread", cmd, Some(Duration::from_millis(200)));
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["--ip", IP, "--port", port.as_str(), "-vv", "bzip2"],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_gzip_compr_level_1() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "gzip",
        "1",
    ]);
    let client_thread = spawn_cmd_thread("Client thread", cmd, Some(Duration::from_millis(200)));
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["--ip", IP, "--port", port.as_str(), "-vv", "gzip"],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_xz_compr_level_1() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "xz",
        "1",
    ]);
    let client_thread = spawn_cmd_thread("Client thread", cmd, Some(Duration::from_millis(200)));
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["--ip", IP, "--port", port.as_str(), "-vv", "xz"],
    );

    let (
        ServerOutput {
            server_stdout: _,
            server_stderr,
        },
        ClientOutput {
            client_stdout: _,
            client_stderr,
        },
    ) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}
