use crate::util::*;
mod util;

const LICENSE: &str = "LICENSE";

#[test]
fn test_evaluate_compression_cargo_toml() -> TestResult {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.args(["evaluate-compression", "--input-file", LICENSE]);

    let StdoutStderr { stdout, stderr } = process_output_to_stdio(cmd.output()?)?;

    eprintln!("{stderr}");
    eprintln!("{stdout}");

    stdout.contains("Best Compression Ratio:   Gzip");
    stdout.contains("Best Compression Time:    Lz4");
    stdout.contains("Best Decompression Time:  Bzip2");

    Ok(())
}
