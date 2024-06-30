// Takes the args and produces a string of the command that should be executed on the remote
// to match the given SendSshArgs
pub(super) fn remote_qft_command_str(
    destination: &str,
    tcp_port: u16,
    verbosity: &str,
    multiple_transfers: bool,
) -> String {
    let mut cmd = String::from("qft listen ");
    cmd.push_str(verbosity);
    cmd.push_str(" --port ");
    cmd.push_str(tcp_port.to_string().as_str());

    if multiple_transfers {
        cmd.push_str(" --output-dir ");
    } else {
        cmd.push_str(" --output ");
    }
    cmd.push_str(destination);

    cmd
}
