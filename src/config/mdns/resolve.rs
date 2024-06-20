use crate::config::util::*;

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsResolveArgs {
    /// mDNS hostname to resolve e.g. `foo` (translates to `foo.local.`)
    pub hostname: String,
    /// Sets a timeout in milliseconds (default 10s)
    #[arg(long, default_value_t = 10000)]
    pub timeout_ms: u64,
    /// Exit as soon as the first IP of the specified hostname has been resolved
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub short_circuit: bool,
}
