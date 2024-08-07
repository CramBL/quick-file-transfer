use std::time::Duration;

use crate::util::*;

#[test]
fn test_qft_mdns_register_resolve() -> TestResult {
    const SERVICE_HOSTNAME: &str = "test_foo";
    const SERVICE_LABEL: &str = "test_label";
    const SERVICE_PROTOCOL: &str = "tcp";
    const SERVICE_INSTANCE_NAME: &str = "test_instance_foo";
    //let service_fullname = format!("{SERVICE_INSTANCE_NAME}._{SERVICE_LABEL}._{SERVICE_PROTOCOL}.local.");
    const KEEP_ALIVE_MS: &str = "600";

    let reg_service_handle = spawn_thread_qft(
        "register service thread",
        [
            "mdns",
            "register",
            "--hostname",
            SERVICE_HOSTNAME,
            "--service-label",
            SERVICE_LABEL,
            "--service-protocol",
            SERVICE_PROTOCOL,
            "--instance-name",
            SERVICE_INSTANCE_NAME,
            "--keep-alive-ms",
            KEEP_ALIVE_MS,
        ],
        None,
    );

    let resolve_hostname_handle = spawn_thread_qft(
        "resolve hostname thread",
        ["mdns", "resolve", SERVICE_HOSTNAME, "--timeout-ms=100"],
        Some(Duration::from_millis(100)),
    );

    let StdoutStderr {
        stdout: _reg_service_stdout,
        stderr: reg_service_stderr,
    } = process_output_to_stdio_if_success(reg_service_handle?.join().unwrap()?)?;

    let StdoutStderr {
        stdout: resolve_hostname_stdout,
        stderr: resolve_hostname_stderr,
    } = process_output_to_stdio_if_success(resolve_hostname_handle?.join().unwrap()?)?;

    assert_no_errors_or_warn(&reg_service_stderr)?;
    assert_no_errors_or_warn(&resolve_hostname_stderr)?;

    assert!(
        resolve_hostname_stdout.contains(&format!("Hostname:  {SERVICE_HOSTNAME}.local.")),
        "Expected stdout to contains {SERVICE_HOSTNAME}. Stdout: {resolve_hostname_stdout}"
    );
    Ok(())
}

#[test]
fn test_qft_mdns_register_discover() -> TestResult {
    const SERVICE_HOSTNAME: &str = "test_foo_alt";
    const SERVICE_LABEL: &str = "test_label";
    const SERVICE_PROTOCOL: &str = "tcp";
    const SERVICE_INSTANCE_NAME: &str = "test_instance_foo";
    //let service_fullname = format!("{SERVICE_INSTANCE_NAME}._{SERVICE_LABEL}._{SERVICE_PROTOCOL}.local.");
    const KEEP_ALIVE_MS: &str = "600";

    let reg_service_handle = spawn_thread_qft(
        "register service thread",
        [
            "mdns",
            "register",
            "--hostname",
            SERVICE_HOSTNAME,
            "--service-label",
            SERVICE_LABEL,
            "--service-protocol",
            SERVICE_PROTOCOL,
            "--instance-name",
            SERVICE_INSTANCE_NAME,
            "--keep-alive-ms",
            KEEP_ALIVE_MS,
        ],
        None,
    );

    let discover_handle = spawn_thread_qft(
        "discover mdns thread",
        [
            "mdns",
            "discover",
            "--service-label",
            SERVICE_LABEL,
            "--service-protocol",
            SERVICE_PROTOCOL,
            "--timeout-ms=100",
        ],
        Some(Duration::from_millis(100)),
    );

    let StdoutStderr {
        stdout: _reg_service_stdout,
        stderr: reg_service_stderr,
    } = process_output_to_stdio_if_success(reg_service_handle?.join().unwrap()?)?;

    let StdoutStderr {
        stdout: discover_stdout,
        stderr: discover_stderr,
    } = process_output_to_stdio_if_success(discover_handle?.join().unwrap()?)?;

    eprintln!("{discover_stdout}");
    eprintln!("{discover_stderr}");

    assert_no_errors_or_warn(&reg_service_stderr)?;
    assert_no_errors_or_warn(&discover_stderr)?;

    assert!(
        discover_stdout.contains(&format!("Hostname:  {SERVICE_HOSTNAME}.local.")),
        "Expected stdout to contains {SERVICE_HOSTNAME}. Stdout: {discover_stdout}"
    );
    Ok(())
}
