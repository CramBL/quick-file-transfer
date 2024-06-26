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
}
