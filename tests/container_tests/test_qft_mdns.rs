use crate::{container_tests::util::*, util::*};

#[test]
#[ignore = "Needs to be run with container test (just d-test)"]
pub fn test_mdns_discover_service_in_container() -> TestResult {
    const SERVICE_HOSTNAME: &str = "container_hostname";
    const SERVICE_LABEL: &str = "container_label";
    const SERVICE_PROTOCOL: &str = "tcp";

    let _test_container = TestContainer::setup(&format!(
        "qft mdns register --hostname {SERVICE_HOSTNAME} --service-label {SERVICE_LABEL} --service-protocol {SERVICE_PROTOCOL} --keep-alive-ms 3000 --color=always"
    ), false);

    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    let args = [
        "mdns",
        "discover",
        "--service-label",
        SERVICE_LABEL,
        "--service-protocol",
        SERVICE_PROTOCOL,
    ];
    cmd.args(args);
    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

    eprint_docker_logs()?;
    eprint_cmd_args_stderr_stdout_formatted(&args, &stdout, &stderr);

    // This error is logged/occurs because of some docker network settings but the service is still correctly discovered so we ignore it.
    let ignore_send_to_interface_error = r"Failed to send to \[";
    assert_no_errors_or_warn_with_ignore(&stderr, &ignore_send_to_interface_error)?;

    assert!(
        stdout.contains(&format!("Hostname:  {SERVICE_HOSTNAME}.local.")),
        "Expected stdout to contains {SERVICE_HOSTNAME}. Stdout: {stdout}"
    );

    Ok(())
}
