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
    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

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
        "27000",
    ];
    cmd.args(args);
    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

    eprint_docker_logs()?;
    eprint_cmd_args_stderr_stdout_formatted(&args, &stdout, &stderr);

    //ERROR Failed to send to [
    assert_no_errors_or_warn(&stderr)?;

    let f = assert_file_exists_in_container(&file_to_receive)?;
    pretty_assert_str_eq!(fs::read_to_string(f)?, TRANSFERED_CONTENTS);

    Ok(())
}
