use super::*;
use testresult::TestResult;

/// matches a single ANSI escape code
pub const ANSI_ESCAPE_REGEX: &str = r"(\x9B|\x1B\[)[0-?]*[ -\/]*[@-~]";
/// WARN prefix with an ANSI escape code
pub const WARN_PREFIX: &str = concat!("WARN ", r"(\x9B|\x1B\[)[0-?]*[ -\/]*[@-~]");
pub const ERROR_PREFIX: &str = concat!("ERROR ", r"(\x9B|\x1B\[)[0-?]*[ -\/]*[@-~]");

/// Helper function to match the raw output of stderr or stdout, with a pattern and return the number of matches
pub fn regex_matches<S>(case_sensitive: bool, haystack: &str, re: S) -> usize
where
    S: AsRef<str> + ToOwned + Display + Into<String>,
{
    // Build regex pattern
    let regex_pattern = if case_sensitive {
        re.to_string()
    } else {
        format!("(?i){re}")
    };
    let re = fancy_regex::Regex::new(&regex_pattern).expect("Failed to compile regex");
    // Count the number of matches
    re.find_iter(haystack).count()
}

/// Helper function to match the raw output of stderr or stdout, with a pattern a fixed amount of times, case insensitive
pub fn match_count<S>(
    case_sensitive: bool,
    haystack: &str,
    re: S,
    expect_match: usize,
) -> TestResult
where
    S: AsRef<str> + ToOwned + Display + Into<String>,
{
    // Build regex pattern
    let regex_pattern = if case_sensitive {
        re.to_string()
    } else {
        format!("(?i){re}")
    };
    let re = fancy_regex::Regex::new(&regex_pattern)?;

    // Count the number of matches
    let match_count = re.find_iter(haystack).count();

    // Assert that the number of matches is equal to the expected number of matches
    pretty_assert_eq!(
        match_count, expect_match,
        "regex: {re} - expected match count: {expect_match}, got {match_count}\nFailed to match on:\n{haystack}"
    );
    Ok(())
}

/// Helper function to match the raw output of stderr or stdout, with a pattern a fixed amount of times, case insensitive
pub fn match_count_with_excludes<S>(
    case_sensitive: bool,
    haystack: &str,
    re: S,
    expect_match: usize,
    exclusions: &[S],
) -> TestResult
where
    S: AsRef<str> + ToOwned + Display + Into<String>,
{
    // Build the base regex pattern
    let base_pattern = re.as_ref().to_string();

    // Add case insensitivity if set
    let regex_pattern = if case_sensitive {
        base_pattern
    } else {
        format!("(?i){base_pattern}")
    };

    // Create the exclusion part of the regex with negative lookaheads
    let exclusions_pattern: String = exclusions
        .iter()
        .map(|ex| format!("(?!.*{}.*$)", ex.as_ref()))
        .collect::<Vec<_>>()
        .join("");

    // Combine the base pattern with exclusions
    let final_pattern = format!("(?m)^{exclusions_pattern}{regex_pattern}.*$");

    // Case sensitivity is already configured so just disable it to prevent `match_count` from including (?i) again
    match_count(false, haystack, final_pattern, expect_match)?;

    Ok(())
}

/// Helper function takes in the output of stderr and asserts that there are no errors, warnings, or thread panics.
pub fn assert_no_errors_or_warn(stderr: &str) -> TestResult {
    match_count(true, stderr, "ERROR", 0)?;
    match_count(true, stderr, "WARN", 0)?;
    match_count(false, stderr, "thread.*panicked", 0)?;
    Ok(())
}

/// Helper function takes in the output of stderr and asserts that there are no errors, warnings, or thread panics. While ignoring an error or warning pattern specified by `ignore`.
pub fn assert_no_errors_or_warn_with_ignore(stderr: &str, ignore: &str) -> TestResult {
    match_count_with_excludes(true, stderr, "ERROR", 0, &[ignore])?;
    match_count_with_excludes(true, stderr, "WARN", 0, &[ignore])?;
    match_count(false, stderr, "thread.*panicked", 0)?;
    Ok(())
}
