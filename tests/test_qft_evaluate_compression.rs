use crate::util::*;
mod util;

const LICENSE: &str = "LICENSE";

#[test]
fn test_evaluate_compression_all() -> TestResult {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.args(["evaluate-compression", "--input-file", LICENSE]);

    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

    eprintln!("{stderr}");
    eprintln!("{stdout}");

    match_count(true, &stderr, "INFO Gzip", 9)?;
    match_count(
        false,
        &stdout,
        r"Best Compression Ratio:.* Gzip\[4\] .* 1\.65:1",
        1,
    )?;

    Ok(())
}

#[test]
fn test_evaluate_compression_omit_bzip2() -> TestResult {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.args([
        "evaluate-compression",
        "--input-file",
        LICENSE,
        "--omit=bzip2",
    ]);

    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

    eprintln!("{stderr}");
    eprintln!("{stdout}");

    match_count(false, &stderr, "Omitting: .*Bzip2", 1)?;
    match_count(false, &stderr, "Bzip2\\[.\\]", 0)?;

    Ok(())
}

#[test]
fn test_evaluate_compression_omit_compression_levels() -> TestResult {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.args([
        "evaluate-compression",
        "--input-file",
        LICENSE,
        "--omit-levels",
        "0",
        "1",
        "2",
        "4",
        "5",
        "6",
        "8",
        "9",
    ]);

    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

    eprintln!("{stderr}");
    eprintln!("{stdout}");

    match_count(
        false,
        &stderr,
        "Omitting compression levels .*0 1 2 4 5 6 8 9",
        1,
    )?;

    match_count(false, &stderr, r"INFO Bzip2", 2)?;
    match_count(false, &stderr, r"INFO Xz", 2)?;
    match_count(false, &stderr, r"INFO Gzip", 2)?;
    match_count(false, &stderr, "Compression level .* 3 ", 3)?;
    match_count(false, &stderr, "Compression level .* 7 ", 3)?;
    Ok(())
}
