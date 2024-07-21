use std::process::ExitCode;

use quick_file_transfer::{config, run};
fn main() -> ExitCode {
    let cfg = match config::Config::init() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("qft: failed at init {e}");
            return ExitCode::FAILURE;
        }
    };
    tracing::trace!("{cfg:?}");

    if let Some(shell) = cfg.completions {
        config::Config::generate_completion_script(shell);
        log::info!("Completions generated for {shell:?}. Exiting...");
        return ExitCode::SUCCESS;
    }

    if let Err(e) = run::run(&cfg) {
        eprintln!("qft: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
