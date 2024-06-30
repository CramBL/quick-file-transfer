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
                use crate::config::transfer::util::{PollAbortCondition, TcpConnectMode};
                use std::path::PathBuf;
                use std::time::Duration;

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

                let input_files: Vec<PathBuf> = args
                    .sources
                    .clone()
                    .into_iter()
                    .map(PathBuf::from)
                    .collect();

                crate::ssh::run_ssh(
                    _cfg,
                    &remote_uri_components.user,
                    args.ssh_private_key_path.as_deref(),
                    args.ssh_key_dir.as_deref(),
                    &remote_uri_components.host,
                    remote_uri_components.destination.to_str().unwrap(),
                    args.ssh_port,
                    args.tcp_port,
                    false,
                    &input_files,
                    true,
                    &args.compression,
                    args.start_port,
                    args.end_port,
                    args.ssh_timeout_ms,
                    args.tcp_delay_ms,
                    TcpConnectMode::poll_from_ms(
                        20_u8,
                        PollAbortCondition::Timeout(Duration::from_secs(10)),
                    ),
                )?;

                Ok(())
            }
        }
    } else {
        bail!("No subcommand specified")
    }
}
