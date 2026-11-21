use std::time::Duration;

/// UDP port the PC listens on for discovery broadcasts.
pub const DISCOVERY_PORT: u16 = 45871;
/// TCP port the PC listens on for commands.
pub const CONTROL_PORT: u16 = 45872;

pub(crate) const DISCOVER: &[u8] = b"mote?";
pub(crate) const HELLO: &str = "mote";
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_millis(600);
pub(crate) const DISCOVER_TIMEOUT: Duration = Duration::from_millis(700);

/// A remote key press, exactly as it reaches Chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Up arrow: previous video.
    Prev,
    /// Down arrow: next video.
    Next,
    /// Right arrow: jump a few seconds forward; repeated while the phone key is held.
    Forward,
    /// F: enter or leave the author's works page.
    Author,
    /// R: not interested.
    Dislike,
    /// X: passed straight through.
    X,
}

impl Command {
    /// Protocol order; the phone lays these out in two columns.
    pub const ALL: [Self; 6] = [
        Self::Prev,
        Self::Next,
        Self::Forward,
        Self::Author,
        Self::Dislike,
        Self::X,
    ];

    /// One command per line, so `printf 'next\n' | nc <pc> 45872` works.
    pub fn wire(self) -> &'static [u8] {
        match self {
            Self::Prev => b"prev\n",
            Self::Next => b"next\n",
            Self::Forward => b"right\n",
            Self::Author => b"author\n",
            Self::Dislike => b"dislike\n",
            Self::X => b"x\n",
        }
    }

    pub fn parse(line: &str) -> Option<Self> {
        match line.trim() {
            "prev" => Some(Self::Prev),
            "next" => Some(Self::Next),
            "right" => Some(Self::Forward),
            "author" => Some(Self::Author),
            "dislike" => Some(Self::Dislike),
            "x" => Some(Self::X),
            _ => None,
        }
    }
}

/// Ports used by both sides. Tests override them.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub discovery_port: u16,
    pub control_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            discovery_port: DISCOVERY_PORT,
            control_port: CONTROL_PORT,
        }
    }
}

/// Parse a discovery reply (`mote <port>\n`) into a control port.
pub(crate) fn parse_hello(bytes: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(bytes).ok()?;
    text.trim().strip_prefix(HELLO)?.trim().parse().ok()
}

pub(crate) fn hello(control_port: u16) -> String {
    format!("{HELLO} {control_port}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_round_trip() {
        for command in Command::ALL {
            let wire = std::str::from_utf8(command.wire()).unwrap();
            assert_eq!(Command::parse(wire), Some(command));
        }
    }

    #[test]
    fn unknown_lines_are_ignored() {
        assert_eq!(Command::parse(""), None);
        assert_eq!(Command::parse("pause"), None);
        assert_eq!(Command::parse("next next"), None);
    }

    #[test]
    fn hello_carries_port() {
        let reply = hello(51234);
        assert_eq!(parse_hello(reply.as_bytes()), Some(51234));
        assert_eq!(parse_hello(b"mote 1\n"), Some(1));
        assert_eq!(parse_hello(b"other 1\n"), None);
        assert_eq!(parse_hello(b"mote\n"), None);
    }
}
