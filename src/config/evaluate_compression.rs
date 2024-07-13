use strum::EnumCount;

use super::{compression::CompressionVariant, util::*};

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct EvaluateCompressionArgs {
    #[arg(short('i'), long)]
    pub input_file: PathBuf,

    /// List of compression formats to omit from evaluation
    #[arg(long, num_args(0..CompressionVariant::COUNT))]
    pub omit: Vec<CompressionVariant>,

    /// List of compression levels to omit from evaluation
    #[arg(long, num_args(0..10))]
    pub omit_levels: Vec<u8>,

    /// The number of threads to use to evaluate compression (1 = sequential), the default is calculated from the available CPUs on the host.
    #[arg(short('j'), long("threads"), value_name("jobs"), default_value_t = default_parallelism())]
    pub threads: usize,

    #[arg(short, long, default_value = "multi")]
    pub progress_bar_mode: ProgressBarMode,
}

fn default_parallelism() -> usize {
    let available: usize = std::thread::available_parallelism()
        .unwrap_or(std::num::NonZeroUsize::new(4).unwrap())
        .into();
    if available > 10 {
        available - 2
    } else if available > 3 {
        available - 1
    } else {
        available
    }
}

#[derive(ValueEnum, Debug, Subcommand, Clone, PartialEq, EnumIter, Display, Copy, Default)]
pub enum ProgressBarMode {
    Single,
    #[default]
    Multi,
}
