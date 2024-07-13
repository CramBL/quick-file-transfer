use crate::config::util::*;

use super::ServiceTypeArgs;

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct MdnsDiscoverArgs {
    #[command(flatten)]
    pub service_type: ServiceTypeArgs,
    /// How long in ms to attempt to discover services before shutdown
    #[arg(long, default_value_t = 1000)]
    pub timeout_ms: u64,
}
