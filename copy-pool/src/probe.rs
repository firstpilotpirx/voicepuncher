//! Whether a copy of a service accepts connections.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// A connection that takes longer than this on loopback counts as refused, so one round
/// over every candidate port fits in the probe interval.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);

/// Tells whether something accepts connections on a port of the service's host.
pub trait Probe: Send + Sync + 'static {
    fn answers(&self, port: u16) -> bool;
}

/// Connects over TCP to the service's host.
pub struct TcpProbe {
    host: String,
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
