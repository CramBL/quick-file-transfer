use crate::config::util::*;

use super::ServiceTypeArgs;

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsDiscoverArgs {
    #[command(flatten)]
    pub service_type: ServiceTypeArgs,
    /// How long in ms to attempt to discover services before shutdown
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,
}
