use crate::util::*;

#[test]
pub fn test_get_version() -> TestResult {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("--version");

    let StdoutStderr { stdout, stderr: _ } = process_output_to_stdio_if_success(cmd.output()?)?;

    pretty_assert_str_eq!(
        stdout,
        format!("Quick File Transfer {}\n", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
pub fn test_generate_completions() -> TestResult {
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("--completions=bash");

    let StdoutStderr { stdout, stderr } = process_output_to_stdio_if_success(cmd.output()?)?;

    assert_no_errors_or_warn(&stderr)?;
    assert!(regex_matches(true, &stdout, r#"opts=".*--version"#) >= 1);
    assert!(regex_matches(true, &stdout, r#"opts=".*--help"#) >= 1);
    Ok(())
}
