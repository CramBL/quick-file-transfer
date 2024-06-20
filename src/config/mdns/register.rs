use crate::config::util::*;

use super::ServiceTypeArgs;

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
pub struct MdnsRegisterArgs {
    /// Service name to register e.g. `foo` (translates to `foo.local.`)
    #[arg(short('n'), long, default_value_t = String::from("test_name"))]
    pub hostname: String,
    #[command(flatten)]
    pub service_type: ServiceTypeArgs,
    #[arg(short, long, default_value_t = String::from("test_inst"))]
    pub instance_name: String,
    /// How long to keep it alive in ms
    #[arg(long, default_value_t = 600000)]
    pub keep_alive_ms: u64,
    /// Service IP, if none provided -> Use auto adressing
    #[arg(long)]
    pub ip: Option<String>,
    /// Service port
    #[arg(long, default_value_t = 11542)]
    pub port: u16,
}
