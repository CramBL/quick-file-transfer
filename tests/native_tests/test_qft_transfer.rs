use fs::read_dir;

use crate::util::*;

pub const IP: &str = "127.0.0.1";

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

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
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

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, server_out.stdout());

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

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;

    pretty_assert_str_eq!(TRANSFERED_CONTENTS, server_out.stdout());

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

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;

    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, server_out.stdout());

    assert!(server_out.success() && client_out.success());

    Ok(())
}

#[test]
pub fn test_file_transfer_no_compression_with_no_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let client_thread = spawn_client_thread(
        file_to_transfer.path(),
        false,
        ["ip", "--no-prealloc", IP, "--port", port.as_str(), "-vv"],
    );

    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["--ip", IP, "--port", port.as_str(), "-vv"],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }

    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);
    assert!(server_out.success() && client_out.success());

    Ok(())
}

#[test]
pub fn test_file_transfer_bzip2_default_with_no_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd_client = Command::cargo_bin(BIN_NAME)?;
    cmd_client.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--no-prealloc",
        "bzip2",
    ]);
    let client_thread = spawn_cmd_thread(
        "Client thread",
        cmd_client,
        Some(Duration::from_millis(200)),
    );
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--decompression",
            "bzip2",
        ],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_gzip_default_with_no_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd_client = Command::cargo_bin(BIN_NAME)?;
    cmd_client.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--no-prealloc",
        "gzip",
    ]);
    let client_thread = spawn_cmd_thread(
        "Client thread",
        cmd_client,
        Some(Duration::from_millis(200)),
    );
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--decompression",
            "gzip",
        ],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_lz4_default_with_no_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd_client = Command::cargo_bin(BIN_NAME)?;
    cmd_client.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--no-prealloc",
        "lz4",
    ]);
    let client_thread = spawn_cmd_thread(
        "Client thread",
        cmd_client,
        Some(Duration::from_millis(200)),
    );
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--decompression",
            "lz4",
        ],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_xz_default_with_no_prealloc() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = dir.child("f2.txt");

    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_0x80000_BYTES;
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();
    let mut cmd_client = Command::cargo_bin(BIN_NAME)?;
    cmd_client.args([
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "--no-prealloc",
        "xz",
    ]);
    let client_thread = spawn_cmd_thread(
        "Client thread",
        cmd_client,
        Some(Duration::from_millis(200)),
    );
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--decompression",
            "xz",
        ],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
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
    let mut cmd_client = Command::cargo_bin(BIN_NAME)?;
    cmd_client.args([
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
    let client_thread = spawn_cmd_thread(
        "Client thread",
        cmd_client,
        Some(Duration::from_millis(200)),
    );
    let server_thread = spawn_server_thread(
        Some(file_to_receive.path()),
        ["--ip", IP, "--port", port.as_str(), "-vv"],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
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
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--decompression=gzip",
        ],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
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
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--decompression",
            "xz",
        ],
    );

    let (server_out, client_out) = join_server_and_client_get_outputs(
        ServerHandle(server_thread?),
        ClientHandle(client_thread?),
    )?;
    if server_out.failed() || client_out.failed() {
        server_out.display_diagnostics();
        client_out.display_diagnostics();
    }
    assert_no_errors_or_warn(server_out.stderr())?;
    assert_no_errors_or_warn(client_out.stderr())?;
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_output_dir_single_file() -> TestResult {
    let fname = "f1.txt";
    // Create a file in temp dir
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child(fname);
    // Create a subdirectory in the temp dir
    let subdir = dir.join("tmp_subdir");
    fs::create_dir(&subdir)?;
    // Get the path and leak it because of lifetimes
    let subdir_path_as_str = subdir.as_path().to_string_lossy().into_owned();
    let subdir_path = subdir_path_as_str.leak();
    // Take the created (empty) subdir and join it with the filename, we expected it to show up here afer the transfer
    let file_to_receive = subdir.join(fname);

    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let port = get_free_port(IP).unwrap();

    let server_thread = spawn_server_thread(
        None,
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--output-dir",
            subdir_path,
        ],
    )?;

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let args = [
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
    ];
    cmd.args(args);
    let StdoutStderr {
        stdout: _client_stdout,
        stderr: client_stderr,
    } = process_output_to_stdio_if_success(cmd.output()?)?;

    let StdoutStderr {
        stdout: _server_stdout,
        stderr: server_stderr,
    } = join_thread_and_get_output_if_success(server_thread)?;

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;

    eprintln!("Dir contents");
    eprintln!("{dir:?}");
    for e in read_dir(subdir)? {
        eprintln!("{e:?}");
    }
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file_to_receive)?);

    Ok(())
}

#[test]
pub fn test_file_transfer_output_dir_multiple_files() -> TestResult {
    let f1_name = "f1.txt";
    let f2_name = "f2.txt";
    // Create a file in temp dir
    let dir = TempDir::new()?;
    let file1_to_transfer = dir.child(f1_name);
    let file2_to_transfer = dir.child(f2_name);
    // Create a subdirectory in the temp dir
    let subdir = dir.join("tmp_subdir");
    fs::create_dir(&subdir)?;
    // Get the path and leak it because of lifetimes
    let subdir_path_as_str = subdir.as_path().to_string_lossy().into_owned();
    let subdir_path = subdir_path_as_str.leak();
    // Take the created (empty) subdir and join it with the filename, we expected it to show up here afer the transfer
    let file1_to_receive = subdir.join(f1_name);
    let file2_to_receive = subdir.join(f2_name);

    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file1_to_transfer, TRANSFERED_CONTENTS)?;
    fs::write(&file2_to_transfer, TRANSFERED_CONTENTS)?;
    assert!(file1_to_transfer.exists());
    assert!(file1_to_transfer.exists());

    let port = get_free_port(IP).unwrap();

    let server_thread = spawn_server_thread(
        None,
        [
            "--ip",
            IP,
            "--port",
            port.as_str(),
            "-vv",
            "--output-dir",
            subdir_path,
        ],
    )?;

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let args = [
        "send",
        "ip",
        IP,
        "--port",
        port.as_str(),
        "-vv",
        "--file",
        file1_to_transfer.path().to_str().unwrap(),
        "--file",
        file2_to_transfer.path().to_str().unwrap(),
    ];
    cmd.args(args);
    let StdoutStderr {
        stdout: _client_stdout,
        stderr: client_stderr,
    } = process_output_to_stdio_if_success(cmd.output()?)?;

    let StdoutStderr {
        stdout: _server_stdout,
        stderr: server_stderr,
    } = join_thread_and_get_output_if_success(server_thread)?;

    eprintln!("=== SERVER ===");
    eprintln!("=== COMMAND STDOUT ===\n{_server_stdout}\n^^^COMMAND STDOUT^^^\n");
    eprintln!("=== COMMAND STDERR ===\n{server_stderr}\n^^^COMMAND STDERR^^^\n");

    eprintln!("=== COMMAND ARGUMENTS ===\n{args:?}\n");
    eprintln!("=== COMMAND STDOUT ===\n{_client_stdout}\n^^^COMMAND STDOUT^^^\n");
    eprintln!("=== COMMAND STDERR ===\n{client_stderr}\n^^^COMMAND STDERR^^^\n");

    assert_no_errors_or_warn(&server_stderr)?;
    assert_no_errors_or_warn(&client_stderr)?;

    eprintln!("Dir contents");
    eprintln!("{dir:?}");
    for e in read_dir(subdir)? {
        eprintln!("{e:?}");
    }
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file1_to_receive)?);
    pretty_assert_str_eq!(TRANSFERED_CONTENTS, fs::read_to_string(file2_to_receive)?);

    Ok(())
}
