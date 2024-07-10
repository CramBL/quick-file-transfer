use crate::{container_tests::util::*, util::*};

#[test]
#[ignore = "Needs to be run with container test (just d-test)"]
pub fn test_ssh_transfer() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f-L,sL.txt");
    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_WHAT;
    fs::write(&file_to_transfer, LOREM_IPSUM_WHAT)?;
    let file_to_receive: String = CONTAINER_HOME_DOWNLOAD_DIR.to_owned() + "/LOREM_WHAT.txt";

    let _test_container = TestContainer::setup("/usr/sbin/sshd -D -p 54320", true);

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let args = [
        "send",
        "ssh",
        "--user",
        CONTAINER_USER,
        "--ip",
        CONTAINER_IP,
        "--ssh-port",
        CONTAINER_SSH_PORT,
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "-vv",
        &format!("--destination={file_to_receive}"),
        "--tcp-port",
        CONTAINER_TCP_PORT,
    ];
    cmd.args(args);
    let StdoutStderr { stdout, stderr } = process_output_to_stdio_if_success(cmd.output()?)?;

    eprint_docker_logs()?;
    eprint_cmd_args_stderr_stdout_formatted(&args, &stdout, &stderr);

    assert_no_errors_or_warn(&stderr)?;
    let f = assert_file_exists_in_container(&file_to_receive)?;
    pretty_assert_str_eq!(fs::read_to_string(f)?, TRANSFERED_CONTENTS);

    Ok(())
}

#[test]
#[ignore = "Needs to be run with container test (just d-test)"]
pub fn test_ssh_transfer_no_tcp_port_specified() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f-L,sL.txt");
    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_WHAT;
    fs::write(&file_to_transfer, LOREM_IPSUM_WHAT)?;
    let file_to_receive: String = CONTAINER_HOME_DOWNLOAD_DIR.to_owned() + "/LOREM_WHAT.txt";

    let _test_container = TestContainer::setup("/usr/sbin/sshd -D -p 54320", true);

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let args = [
        "send",
        "ssh",
        "--user",
        CONTAINER_USER,
        "--ip",
        CONTAINER_IP,
        "--ssh-port",
        CONTAINER_SSH_PORT,
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "-vv",
        &format!("--destination={file_to_receive}"),
        "--start-port",
        CONTAINER_DYNAMIC_PORTS_START,
        "--end-port",
        CONTAINER_DYNAMIC_PORTS_END,
    ];
    cmd.args(args);
    let StdoutStderr { stdout, stderr } = process_output_to_stdio_if_success(cmd.output()?)?;

    eprint_docker_logs()?;
    eprint_cmd_args_stderr_stdout_formatted(&args, &stdout, &stderr);

    //ERROR Failed to send to [
    assert_no_errors_or_warn(&stderr)?;

    let f = assert_file_exists_in_container(&file_to_receive)?;
    pretty_assert_str_eq!(fs::read_to_string(f)?, TRANSFERED_CONTENTS);

    Ok(())
}

#[test]
#[ignore = "Needs to be run with container test (just d-test)"]
pub fn test_ssh_transfer_no_tcp_port_specified_scp_args() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f-L,sL.txt");
    const TRANSFERED_CONTENTS: &str = LOREM_IPSUM_WHAT;
    fs::write(&file_to_transfer, LOREM_IPSUM_WHAT)?;
    let file_to_receive: String = CONTAINER_HOME_DOWNLOAD_DIR.to_owned() + "/LOREM_WHAT.txt";

    let _test_container = TestContainer::setup("/usr/sbin/sshd -D -p 54320", true);

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let args = [
        "send",
        "ssh",
        &format!("{CONTAINER_USER}@{CONTAINER_IP}:{file_to_receive}"),
        "--ssh-port",
        CONTAINER_SSH_PORT,
        "--file",
        file_to_transfer.path().to_str().unwrap(),
        "-vv",
        "--start-port",
        CONTAINER_DYNAMIC_PORTS_START,
        "--end-port",
        CONTAINER_DYNAMIC_PORTS_END,
    ];
    cmd.args(args);
    let StdoutStderr { stdout, stderr } = process_output_to_stdio_if_success(cmd.output()?)?;

    eprint_docker_logs()?;
    eprint_cmd_args_stderr_stdout_formatted(&args, &stdout, &stderr);

    //ERROR Failed to send to [
    assert_no_errors_or_warn(&stderr)?;

    let f = assert_file_exists_in_container(&file_to_receive)?;
    pretty_assert_str_eq!(fs::read_to_string(f)?, TRANSFERED_CONTENTS);

    Ok(())
}

#[test]
#[ignore = "Needs to be run with container test (just d-test)"]
pub fn test_ssh_transfer_no_tcp_port_specified_multiple_files() -> TestResult {
    let f1name = "f1-L,sL11.txt";
    let f2name = "f2-L,s22L.txt";
    let subdir_name = "transfer_subdir";
    let dir = TempDir::new()?;
    let file1_to_transfer = dir.child(f1name);
    let file2_to_transfer = dir.child(f2name);

    const TRANSFERED_CONTENTS1: &str = LOREM_IPSUM_WHAT;
    const TRANSFERED_CONTENTS2: &str = LOREM_IPSUM_WHERE;
    fs::write(&file1_to_transfer, TRANSFERED_CONTENTS1)?;
    fs::write(&file2_to_transfer, TRANSFERED_CONTENTS2)?;
    let subdir = PathBuf::from(CONTAINER_HOME_DOWNLOAD_DIR).join(subdir_name);
    let file1_to_receive: String =
        CONTAINER_HOME_DOWNLOAD_DIR.to_owned() + "/" + subdir_name + "/" + f1name;
    let file2_to_receive: String =
        CONTAINER_HOME_DOWNLOAD_DIR.to_owned() + "/" + subdir_name + "/" + f2name;
    let subdir_as_str: String = subdir.to_str().unwrap().to_owned();

    let _test_container = TestContainer::setup("/usr/sbin/sshd -D -p 54320", true);

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let args = [
        "send",
        "ssh",
        "--user",
        CONTAINER_USER,
        "--ip",
        CONTAINER_IP,
        "--ssh-port",
        CONTAINER_SSH_PORT,
        "--file",
        file1_to_transfer.path().to_str().unwrap(),
        "--file",
        file2_to_transfer.path().to_str().unwrap(),
        "-vv",
        &format!("--destination={subdir_as_str}"),
        "--start-port",
        CONTAINER_DYNAMIC_PORTS_START,
        "--end-port",
        CONTAINER_DYNAMIC_PORTS_END,
    ];
    cmd.args(args);
    let StdoutStderr { stdout, stderr } = process_output_to_stdio_if_success(cmd.output()?)?;

    eprint_docker_logs()?;
    eprint_cmd_args_stderr_stdout_formatted(&args, &stdout, &stderr);

    //ERROR Failed to send to [
    assert_no_errors_or_warn(&stderr)?;

    let f1 = assert_file_exists_in_container(&file1_to_receive)?;
    let f2 = assert_file_exists_in_container(&file2_to_receive)?;
    pretty_assert_str_eq!(fs::read_to_string(f1)?, TRANSFERED_CONTENTS1);
    pretty_assert_str_eq!(fs::read_to_string(f2)?, TRANSFERED_CONTENTS2);

    Ok(())
}
