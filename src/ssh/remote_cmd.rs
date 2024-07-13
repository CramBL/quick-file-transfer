// Takes the args and produces a string of the command that should be executed on the remote
// to match the given SendSshArgs
pub(super) fn remote_qft_command_str(tcp_port: u16, verbosity: &str) -> String {
    let mut cmd = String::from("qft listen --remote ");
    cmd.push_str(verbosity);
    cmd.push_str(" --port ");
    cmd.push_str(tcp_port.to_string().as_str());
    cmd
}
