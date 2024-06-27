use anyhow::bail;

use crate::config::{Command, Config};

pub fn run(_cfg: &Config) -> anyhow::Result<()> {
    if let Some(ref cmd) = _cfg.command {
        match cmd {
            Command::Listen(ref args) => crate::server::listen(_cfg, args),
            Command::Send(ref cmd) => crate::send::handle_send_cmd(cmd, _cfg),

            #[cfg(feature = "mdns")]
            Command::Mdns(ref cmd) => crate::mdns::handle_mdns_command(&cmd.subcmd),

            #[cfg(feature = "evaluate-compression")]
            Command::EvaluateCompression(ref args) => {
                crate::evaluate_compression::evaluate_compression(args.clone())
            }

            Command::GetFreePort(ref a) => crate::get_free_port::handle_get_free_port(a),
        }
    } else {
        bail!("No subcommand specified")
    }
}
