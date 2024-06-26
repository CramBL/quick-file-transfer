use crate::config::{Command, Config};

pub fn run(cfg: Config) -> anyhow::Result<()> {
    match cfg.command {
        Command::Listen(ref args) => crate::server::listen(&cfg, args),
        Command::Send(ref cmd) => crate::send::handle_send_cmd(cmd, &cfg),

        #[cfg(feature = "mdns")]
        Command::Mdns(cmd) => crate::mdns::handle_mdns_command(cmd.subcmd),

        #[cfg(feature = "evaluate-compression")]
        Command::EvaluateCompression(args) => {
            crate::evaluate_compression::evaluate_compression(args)
        }

        Command::GetFreePort(ref a) => crate::get_free_port::handle_get_free_port(a),
    }
}
