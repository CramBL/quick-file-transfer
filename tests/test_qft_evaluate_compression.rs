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

    match_count(false, &stderr, "Gzip\\[9\\]", 1)?;
    match_count(false, &stdout, "Best Compression Ratio:.* Gzip\\[4\\]", 1)?;

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

    // Checks that there's two matches on either Xz[3] or Xz[7] (technically two of the same would pass)
    match_count(false, &stderr, "Xz\\[[37]\\]", 2)?;
    // Checks that there's 0 instances of Xz[#] where # is one of the class [1245689]
    match_count(false, &stderr, "Xz\\[[1245689]\\]", 0)?;
    match_count(false, &stderr, "Gzip\\[[37]\\]", 2)?;
    match_count(false, &stderr, "Gzip\\[[1245689]\\]", 0)?;
    match_count(false, &stderr, "Bzip2\\[[3]\\]", 1)?; // Check specifically for Bzip[3]
    match_count(false, &stderr, "Bzip2\\[[7]\\]", 1)?; // Check specifically for Bzip[7]
    match_count(false, &stderr, "Bzip2\\[[1245689]\\]", 0)?;

    Ok(())
}
