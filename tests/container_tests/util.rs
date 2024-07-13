use crate::util::*;
use anyhow::{bail, Result};
use std::process::Command;
use std::sync::OnceLock;

pub const CONTAINER_IP: &str = "127.0.0.1";
pub const CONTAINER_SSH_PORT: &str = "54320";
pub const CONTAINER_TCP_PORT: &str = "12999";
pub const CONTAINER_DYNAMIC_PORTS_START: &str = "49152";
pub const CONTAINER_DYNAMIC_PORTS_END: &str = "49154";
pub const CONTAINER_USER: &str = "userfoo";
pub const CONTAINER_HOME_DOWNLOAD_DIR: &str = "/home/userfoo/downloads";

const JUST_CONTAINER_STOP_RECIPE: &str = "d-stop";

const TEST_TMP_DIR: &str = "docker_mounted_tmp/";

static SETUP_ONCE: OnceLock<()> = OnceLock::new();

fn setup_test_container(command: &str, args: &[&str], include_ssh_keys: bool) -> StdoutStderr {
    SETUP_ONCE.get_or_init(|| {
        // Cleanup routine is stopping the container if it isn't stopped and cleanup of the mounted tmp dir (if it exists)
        run_stop_container_recipe().expect("Failed running stop container recipe");
        cleanup_mounted_tmp_dir().expect("failed cleaning mounted tmp dir");
    });
    let output = Command::new(command).args(args).output().unwrap();
    if include_ssh_keys {
        let out = run_just_cmd("d-setup-ssh-login", [""]).unwrap();
        eprintln!(
            "Include ssh keys (d-setup-ssh-login)\n===> STDOUT\n{}\n===> STDERR\n{}\n",
            out.stdout, out.stderr
        );
    }
    process_output_to_stdio_if_success(output).unwrap()
}

fn run_stop_container_recipe() -> Result<()> {
    let output = Command::new("just")
        .args([JUST_CONTAINER_STOP_RECIPE])
        .output()?;
    let StdoutStderr { stdout, stderr } = process_output_to_stdio_if_success(output)?;

    eprintln!("===> Cleanup ({JUST_CONTAINER_STOP_RECIPE}) STDOUT:\n{stdout}\n");
    eprintln!("===> Cleanup ({JUST_CONTAINER_STOP_RECIPE}) STDERR:\n{stderr}\n");
    Ok(())
}

fn cleanup_mounted_tmp_dir() -> Result<()> {
    let tmp_path = PathBuf::from(TEST_TMP_DIR);
    assert!(
        tmp_path.exists(),
        "The tmp directory {tmp_path:?} that docker mounts does not exist!"
    );
    for res in fs::read_dir(tmp_path)? {
        match res {
            Ok(dir_entry) => match dir_entry.file_type() {
                Ok(t) => {
                    let entry_path = dir_entry.path();
                    if t.is_file() {
                        fs::remove_file(entry_path)?;
                    } else if t.is_dir() {
                        fs::remove_dir_all(entry_path)?;
                    }
                }
                Err(e) => eprintln!("{e}"),
            },
            Err(e) => eprintln!("{e}"),
        }
    }
    Ok(())
}

fn perform_cleanup() -> Result<()> {
    // Run the cleanup command
    let output = Command::new("just")
        .args([JUST_CONTAINER_STOP_RECIPE])
        .output()?;
    let StdoutStderr { stdout, stderr } = process_output_to_stdio_if_success(output)?;

    eprintln!("===> Cleanup ({JUST_CONTAINER_STOP_RECIPE}) STDOUT:\n{stdout}\n");
    eprintln!("===> Cleanup ({JUST_CONTAINER_STOP_RECIPE}) STDERR:\n{stderr}\n");

    cleanup_mounted_tmp_dir()?;
    Ok(())
}

pub struct TestContainer {
    pub stdout_stderr: StdoutStderr,
}

impl TestContainer {
    pub fn setup(args: &str, include_ssh_keys: bool) -> Self {
        // Using the test container requires setting RUST_TEST_THREADS=1 or NEXTEST_TEST_THREADS=1 if using nex test
        if std::env::var_os("NEXTEST").is_some() {
            let tt = std::env::var_os("NEXTEST_TEST_THREADS");
            assert!(tt.is_some(), "It appears you are using nex test to run the container tests but didn't set NEXTEST_TEST_THREADS=1");
            assert_eq!(tt.unwrap(), "1", "It appears you are using nex test to run the container tests, You need to set NEXTEST_TEST_THREADS=1");
        } else if let Some(tt) = std::env::var_os("RUST_TEST_THREADS") {
            assert_eq!(
                tt, "1",
                "Running tests ussing the test container requires setting RUST_TEST_THREADS=1"
            )
        } else {
            panic!("Running tests using the test container requires setting RUST_TEST_THREADS=1 or NEXTEST_TEST_THREADS=1 if using nextest");
        }

        let stdout_stderr = setup_test_container("just", &["d-run-with", args], include_ssh_keys);

        Self { stdout_stderr }
    }

    #[allow(unused)]
    pub fn stdout(&self) -> &str {
        &self.stdout_stderr.stdout
    }

    #[allow(unused)]
    pub fn stderr(&self) -> &str {
        &self.stdout_stderr.stderr
    }
}

impl Drop for TestContainer {
    fn drop(&mut self) {
        perform_cleanup().expect("Test container cleanup failed!");
    }
}

/// Executes `Just` with `recipe` passing args to the command/recipe.
/// asserts that it returned status code 0 and returns the stdout/stderr command output
pub fn run_just_cmd<I, S>(recipe: &str, args: I) -> Result<StdoutStderr>
where
    I: IntoIterator<Item = S> + Send + 'static + Debug,
    S: ToOwned + AsRef<std::ffi::OsStr>,
    String: FromIterator<S>,
{
    // Collect the arguments into a single string as most recipes expect the arguments as just one string/arg
    let mut cmd = Command::new("just");
    cmd.arg(recipe);
    let args_str: String = args.into_iter().collect();
    if !args_str.is_empty() {
        cmd.arg(args_str);
    }

    let output = cmd.output()?;
    process_output_to_stdio_if_success(output)
}

fn check_and_relocate_path(original_path: &Path) -> Result<PathBuf> {
    let container_home_download_dir = Path::new(CONTAINER_HOME_DOWNLOAD_DIR);
    let test_tmp_dir = Path::new(TEST_TMP_DIR);

    if original_path.starts_with(container_home_download_dir) {
        // Remove the CONTAINER_HOME_DOWNLOAD_DIR part
        let remainder = original_path
            .strip_prefix(container_home_download_dir)
            .unwrap();

        // Append the remainder to TEST_TMP_DIR
        let new_path = test_tmp_dir.join(remainder);

        // Check if the new path exists
        assert!(new_path.exists(), "Path does not exist: {new_path:?}");
        Ok(new_path)
    } else {
        bail!("Path does not start with {CONTAINER_HOME_DOWNLOAD_DIR}: {original_path:?}");
    }
}
/// Asserts that the file exists in the temp directory that was mounted into the container
/// if the assertion is true, returns the path to the file
pub fn assert_file_exists_in_container(path: &str) -> Result<PathBuf> {
    let tmp_dir = PathBuf::from(TEST_TMP_DIR);
    assert!(tmp_dir.exists() && tmp_dir.is_dir());
    let p = PathBuf::from(path);
    check_and_relocate_path(&p)
}

pub fn get_docker_logs() -> Result<StdoutStderr> {
    run_just_cmd("d-logs", [""])
}

pub fn eprint_docker_logs() -> Result<()> {
    let StdoutStderr { stdout, stderr } = get_docker_logs()?;
    eprintln!("====== DOCKER LOGS ======:\n===> STDOUT\n{stdout}\n===> STDERR\n{stderr}\n^^^^^^^^^^^^^^^^^^^^^^^^\n       DOCKER LOGS\n\n",);
    Ok(())
}

/// Print args and stdout and stderr in a way that is easy to parse in the terminal (for debugging)
pub fn eprint_cmd_args_stderr_stdout_formatted(args: &[&str], stdout: &str, stderr: &str) {
    eprintln!("=== COMMAND ARGUMENTS ===\n{args:?}\n");
    eprintln!("=== COMMAND STDOUT ===\n{stdout}\n^^^COMMAND STDOUT^^^\n");
    eprintln!("=== COMMAND STDERR ===\n{stderr}\n^^^COMMAND STDERR^^^\n");
}
