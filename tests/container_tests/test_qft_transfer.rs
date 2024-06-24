pub const IP: &str = "127.0.0.1";
pub const CONTAINER_PORT: &str = "12999";

use crate::{
    container_tests::{assert_file_exists_in_container, eprint_docker_logs, TestContainer},
    util::*,
};

#[test]
#[ignore = "Needs to be run with container test (just d-test)"]
pub fn test_file_transfer_no_compression_simple() -> TestResult {
    let dir = TempDir::new()?;
    let file_to_transfer = dir.child("f1.txt");
    let file_to_receive = "/tmp/received.txt";

    const TRANSFERED_CONTENTS: &str = "contents";
    fs::write(&file_to_transfer, TRANSFERED_CONTENTS)?;

    let test_container_cmd =
        format!("qft listen --port {CONTAINER_PORT} -vv --file {file_to_receive}");
    let _test_container = TestContainer::setup(&test_container_cmd);

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.args([
        "send",
        "ip",
        IP,
        "--port",
        CONTAINER_PORT,
        "-vv",
        "--file",
        file_to_transfer.path().to_str().unwrap(),
    ]);
    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

    eprint_docker_logs()?;
    eprintln!("{stderr}");
    eprintln!("{stdout}");

    let f = assert_file_exists_in_container(file_to_receive)?;
    pretty_assert_str_eq!(fs::read_to_string(f)?, TRANSFERED_CONTENTS);

    Ok(())
}
