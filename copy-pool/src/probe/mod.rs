//! Whether a copy of a service accepts connections, and whether the program on its port is
//! the service.

use std::fmt;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

mod holder;
mod whisper;

pub use holder::holder_of;
pub use whisper::WhisperProbe;

/// A connection that takes longer than this on loopback counts as refused, so one round
/// over every candidate port fits in the probe interval.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);

/// What the program that accepts connections on a port says it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// The service: the port may take requests.
    Service,
    /// Another program: the port never takes a request.
    Foreign(ForeignProgram),
    /// No answer in time: asked again at the next probe round.
    Unknown,
}

/// A program that is not the service, listening on a port of the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignProgram {
    pub port: u16,
    /// The program's name, when the system tells it.
    pub name: Option<String>,
    pub pid: Option<u32>,
}

impl fmt::Display for ForeignProgram {
    /// The program as a person names it: "app (pid 28344)".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.name, self.pid) {
            (Some(name), Some(pid)) => write!(f, "{name} (pid {pid})"),
            (Some(name), None) => write!(f, "{name}"),
            (None, Some(pid)) => write!(f, "a program (pid {pid})"),
            (None, None) => write!(f, "an unknown program"),
        }
    }
}

/// Tells whether something accepts connections on a port of the service's host, and what it is.
pub trait Probe: Send + Sync + 'static {
    fn answers(&self, port: u16) -> bool;

    /// What the program on `port` says it is; asked once when the port starts accepting
    /// connections. A probe that cannot tell takes every program for the service.
    fn identify(&self, _port: u16) -> Identity {
        Identity::Service
    }
}

/// Connects over TCP to the service's host.
pub struct TcpProbe {
    pub(super) host: String,
}

impl TcpProbe {
    pub fn new(host: &str) -> Self {
        Self { host: host.to_owned() }
    }
}

impl Probe for TcpProbe {
    fn answers(&self, port: u16) -> bool {
        (self.host.as_str(), port)
            .to_socket_addrs()
            .map(|mut addrs| {
                addrs.any(|addr| TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok())
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn should_tell_a_listening_port_from_a_closed_one() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let open = listener.local_addr().unwrap().port();
        let closed = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let probe = TcpProbe::new("127.0.0.1");
        assert!(probe.answers(open));
        assert!(!probe.answers(closed));
    }
}
