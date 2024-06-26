// Takes the args and produces a string of the command that should be executed on the remote
// to match the given SendSshArgs
pub(super) fn remote_qft_command_str(
    destination: &str,
    tcp_port: u16,
    prealloc: bool,
    compression: Option<&crate::config::compression::Compression>,
    verbosity: &str,
) -> String {
    let mut cmd = String::from("qft listen ");
    cmd.push_str(verbosity);
    cmd.push_str(" --port ");
    cmd.push_str(tcp_port.to_string().as_str());

    if prealloc {
        cmd.push_str(" --prealloc");
    }
    cmd.push_str(" --output ");
    cmd.push_str(destination);

    if let Some(compression) = compression {
        cmd.push(' ');
        cmd.push_str(compression.variant_as_str());
    }
    cmd
}
