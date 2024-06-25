use crate::config::{compression::CompressionVariant, util::*};

/// Holds the Listen subcommands
#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct ListenArgs {
    /// Host IP e.g. `127.0.0.1`
    #[arg(long, default_value_t  = String::from("0.0.0.0"))]
    pub ip: String,
    /// e.g. 30301
    #[arg(short, long, default_value_t = 12993)]
    pub port: u16,

    /// Supply a path for outputting contents (if none: use stdio)
    #[arg(short, long, value_name("OUTPUT_PATH"), name("OUTPUT"), global(true))]
    pub output: Option<PathBuf>,

    /// Client will send the size of the file to the server allowing the server to preallocate for the expected size
    #[arg(long, action = ArgAction::SetTrue, requires = "OUTPUT", global(true))]
    pub prealloc: bool,

    /// Compression format of the received file
    #[command(subcommand)]
    pub compression: Option<CompressionVariant>,
}
