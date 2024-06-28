use crate::config::{compression::CompressionVariant, util::*};

/// Holds the Listen subcommands
#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct ListenArgs {
    /// Host IP e.g. `127.0.0.1` for localhost or 0.0.0.0 for any address.
    #[arg(long, default_value_t  = String::from("0.0.0.0"))]
    pub ip: String,

    /// Prefer ports 49152-65535 as ports outside that range may be reserved.
    #[arg(short, long, default_value = Some("49152"), value_parser = clap::value_parser!(u16).range(1024..),
        long_help("Specify port to listen on, e.g. 49999. \
        Prefer ports 49152-65535 as ports outside that range may be reserved, while 49152 and higher are for dynamic use. \
        \n0-1024 are reserved for special purposes and require root, 1024-49152 can be reserved for various services. \
        \nHigher numbers are preferred for ephemeral/dynamic use (client processes), such as temporary outgoing ssh connections and the likes.")
    )]
    pub port: Option<u16>,

    /// Supply a path for outputting contents (if none: use stdio)
    #[arg(short('o'), long, value_name("PATH"), global(true), group("OUTPUT"))]
    pub output: Option<PathBuf>,

    /// Specify a directory for storing received files. Files retain their original name from the client host.
    #[arg(
        long("output-dir"),
        value_name("DIRECTORY"),
        global(true),
        group("OUTPUT")
    )]
    pub output_dir: Option<PathBuf>,

    /// Compression format of the received file, incremental decompression is performed as the data is received.
    #[arg(short('d'), long, global(true))]
    pub decompression: Option<CompressionVariant>,
}
