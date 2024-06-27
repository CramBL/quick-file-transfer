use super::util::*;

/// Styling for the `help` terminal output
pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorWhen {
    Always,
    Auto,
    Never,
}

impl std::fmt::Display for ColorWhen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

/// Used to choose IP version for any commands where that is appropriate
#[derive(Debug, Default, ValueEnum, Clone, Copy)]
pub enum IpVersion {
    #[default]
    V4,
    V6,
}

impl fmt::Display for IpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpVersion::V4 => write!(f, "v4"),
            IpVersion::V6 => write!(f, "v6"),
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TransportLayerProtocol {
    TCP,
    UDP,
}

impl fmt::Display for TransportLayerProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportLayerProtocol::TCP => write!(f, "tcp"),
            TransportLayerProtocol::UDP => write!(f, "udp"),
        }
    }
}
