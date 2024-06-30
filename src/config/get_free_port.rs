use crate::config::util::*;

#[derive(Debug, Args, Clone)]
#[command(flatten_help = true)]
pub struct GetFreePortArgs {
    /// Host IP e.g. `127.0.0.1` for localhost
    #[arg(default_value_t  = String::from("0.0.0.0"), value_parser = parse_valid_ip)]
    pub ip: String,

    /// Start of the port range e.g. 50000. IANA recommends: 49152-65535 for dynamic use.
    #[arg(short, long)]
    pub start_port: Option<u16>,

    /// End of the port range e.g. 51000. IANA recommends: 49152-65535 for dynamic use.
    #[arg(short, long, requires("start_port"))]
    pub end_port: Option<u16>,
}

fn parse_valid_ip(ip_str: &str) -> anyhow::Result<String> {
    if ip_str.parse::<std::net::IpAddr>().is_err() {
        anyhow::bail!("'{ip_str}' is not a valid IP address.")
    }
    Ok(ip_str.to_owned())
}
