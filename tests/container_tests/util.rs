use crate::util::*;
use anyhow::Result;
use std::process::Command;
use std::sync::OnceLock;

const TEST_TMP_DIR: &str = "target/docker_mounted_tmp/";

static SETUP_ONCE: OnceLock<()> = OnceLock::new();

fn setup_test_container(command: &str, args: &[&str]) -> StdoutStderr {
    SETUP_ONCE.get_or_init(|| {
        // Cleanup routine is stopping the container if it isn't stopped and cleanup of the mounted tmp dir (if it exists)
        run_stop_container_recipe().expect("Failed running stop container recipe");
        cleanup_mounted_tmp_dir().expect("failed cleaning mounted tmp dir");
    });
    let output = Command::new(command).args(args).output().unwrap();
    process_output_to_stdio(output).unwrap()
}

fn run_stop_container_recipe() -> Result<()> {
    let output = Command::new("just").args(["d-stop"]).output()?;
    let StdoutStderr { stdout, stderr } = process_output_to_stdio(output)?;

    eprintln!("Cleanup stdout: {stdout}");
    eprintln!("Cleanup stderr: {stderr}");
    Ok(())
}

fn cleanup_mounted_tmp_dir() -> Result<()> {
    let tmp_path = PathBuf::from(format!("{TEST_TMP_DIR}/tmp"));
    if tmp_path.exists() {
        fs::remove_dir_all(tmp_path.as_path())?;
    }
    fs::create_dir(tmp_path)?;
    Ok(())
}

fn perform_cleanup() -> Result<()> {
    // Run the cleanup command
    let output = Command::new("just").args(["d-stop"]).output()?;
    let StdoutStderr { stdout, stderr } = process_output_to_stdio(output)?;

    eprintln!("Cleanup stdout: {stdout}");
    eprintln!("Cleanup stderr: {stderr}");

    let tmp_path = PathBuf::from(format!("{TEST_TMP_DIR}/tmp"));
    fs::remove_dir_all(tmp_path.as_path())?;
    fs::create_dir(tmp_path)?;
    Ok(())
}

pub struct TestContainer {
    pub stdout_stderr: StdoutStderr,
}

impl TestContainer {
    pub fn setup(args: &str) -> Self {
        // Using the test container requires setting RUST_TEST_THREADS=1
        let tt = std::env::var_os("RUST_TEST_THREADS");
        assert!(
            tt.is_some(),
            "Running tests using the test container requires setting RUST_TEST_THREADS=1"
        );
        assert_eq!(
            tt.unwrap(),
            "1",
            "Running tests sing the test container requires setting RUST_TEST_THREADS=1"
        );

        let stdout_stderr = setup_test_container("just", &["d-run-with", args]);

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
        perform_cleanup().expect("Test container clenup failed!");
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
    process_output_to_stdio(output)
}

/// Asserts that the file exists in the temp directory that was mounted into the container
/// if the assertion is true, returns the path to the file
pub fn assert_file_exists_in_container(path: &str) -> Result<PathBuf> {
    let correcte_path: PathBuf = PathBuf::from(format!("{TEST_TMP_DIR}/{path}"));
    assert!(correcte_path.exists(), "{correcte_path:?} doesn't exist!");
    Ok(correcte_path)
}

pub fn get_docker_logs() -> Result<StdoutStderr> {
    run_just_cmd("d-logs", [""])
}

pub fn eprint_docker_logs() -> Result<()> {
    let StdoutStderr { stdout, stderr } = get_docker_logs()?;
    eprintln!("====== DOCKER LOGS ======:\n===> STDOUT\n{stdout}\n===>STDERR\n{stderr}\n^^^^^^^^^^^^^^^^^^^^^^^^\n       DOCKER LOGS\n\n",);
    Ok(())
}
