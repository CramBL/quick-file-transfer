use super::util::*;

pub mod listen;
pub mod send;

#[derive(Debug, Args, Clone)]
pub struct ContentTransferArgs {
    /// Compression format
    #[command(subcommand)]
    compression: Option<Compression>,

    /// Supply a file for I/O (if none: use stdio)
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Client will send the size of the file to the server allowing the server to preallocate for the expected size
    #[arg(long, action = ArgAction::SetTrue, requires = "file")]
    prealloc: bool,
}

impl ContentTransferArgs {
    pub fn compression(&self) -> Option<Compression> {
        self.compression
    }
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }
    pub fn prealloc(&self) -> bool {
        self.prealloc
    }
}
