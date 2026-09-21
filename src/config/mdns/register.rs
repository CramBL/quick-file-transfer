use crate::config::util::*;

use super::ServiceTypeArgs;

fn parse_duration_ms(s: &str) -> Result<u64, String> {
    let (number, multiplier) = if let Some(number) = s.strip_suffix('s') {
        (number, 1_000)
    } else if let Some(number) = s.strip_suffix('m') {
        (number, 60_000)
    } else if let Some(number) = s.strip_suffix('h') {
        (number, 3_600_000)
    } else {
        (s, 1)
    };

    number
        .parse::<u64>()
        .map_err(|_| format!("invalid duration: {s}"))?
        .checked_mul(multiplier)
        .ok_or_else(|| format!("duration too large: {s}"))
}

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct MdnsRegisterArgs {
    /// Service name to register e.g. `foo` (translates to `foo.local.`)
    #[arg(short('n'), long, default_value_t = String::from("test_name"))]
    pub hostname: String,
    #[command(flatten)]
    pub service_type: ServiceTypeArgs,
    #[arg(short, long, default_value_t = String::from("test_inst"))]
    pub instance_name: String,
    /// How long to keep it alive in milliseconds, or suffix with s, m, or h
    #[arg(long, default_value_t = 600000, value_parser = parse_duration_ms)]
    pub keep_alive_ms: u64,
    /// Service IP, if none provided -> Use auto adressing
    #[arg(long)]
    pub ip: Option<String>,
    /// Service port
    #[arg(long, default_value_t = 11542)]
    pub port: u16,
}
