use crate::config::{compression::CompressionVariant, util::*};

/// Holds the Listen subcommands
#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct ListenArgs {
    /// Host IP e.g. `127.0.0.1` for localhost or 0.0.0.0 for any address.
    #[arg(long, default_value_t  = String::from("0.0.0.0"))]
    pub ip: String,

    /// e.g. 30301
    #[arg(short, long, default_value_t = 12993,
        long_help("Specify port to listen on, e.g. 30301. \
        Prefer ports 1024-49151 as ports below that number are reserved for special use. \
        \nHigher numbers are preferred for ephemeral/dynamic use (client processes), such as temporary outgoing ssh connections and the likes.")
    )]
    pub port: u16,

    /// Supply a path for outputting contents (if none: use stdio)
    #[arg(short('o'), long, value_name("PATH"), name("OUTPUT"), global(true))]
    pub output: Option<PathBuf>,

    /// Specify that a client is first sending information about the size of the file to the server, allowing the server to preallocate for the expected size
    #[arg(long("prealloc"), action = ArgAction::SetTrue, requires = "OUTPUT", global(true))]
    pub prealloc: bool,

    /// Compression format of the received file, incremental decompression is performed as the data is received.
    #[arg(short('d'), long, global(true))]
    pub decompression: Option<CompressionVariant>,
}
