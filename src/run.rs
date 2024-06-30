use anyhow::bail;

use crate::config::{Command, Config};

pub fn run(_cfg: &Config) -> anyhow::Result<()> {
    if let Some(ref cmd) = _cfg.command {
        match cmd {
            Command::Listen(ref args) => crate::server::listen(_cfg, args),
            Command::Send(ref cmd) => crate::send::handle_send_cmd(cmd, _cfg),
            Command::GetFreePort(ref a) => crate::get_free_port::handle_get_free_port(a),

            #[cfg(feature = "mdns")]
            Command::Mdns(ref cmd) => crate::mdns::handle_mdns_command(&cmd.subcmd),

            #[cfg(feature = "evaluate-compression")]
            Command::EvaluateCompression(ref args) => {
                crate::evaluate_compression::evaluate_compression(args.clone())
            }

            #[cfg(feature = "ssh")]
            Command::Ssh(ref args) => {
                // Determine if the operation is local to remote or remote to local
                let is_local_to_remote = args.is_sending();
                let is_remote_to_local = !args.is_sending();

                let remote_uri_components = if is_local_to_remote {
                    crate::config::ssh::parse_scp_style_uri(&args.destination)
                } else {
                    crate::config::ssh::parse_scp_style_uri(args.sources.first().unwrap())
                }?;
                println!("URI: {remote_uri_components:?}");

                println!("Sources: {:?}", args.sources);
                println!("Destination: {}", args.destination);
                //println!("Recursive: {}", args.recursive);
                //println!("Preserve Times: {}", args.preserve_times);
                //println!("Verbose: {}", args.verbose);
                println!(
                    "Operation: {}",
                    if is_remote_to_local {
                        "Remote to Local"
                    } else {
                        "Local to Remote"
                    }
                );
                Ok(())
            }
        }
    } else {
        bail!("No subcommand specified")
    }
}
