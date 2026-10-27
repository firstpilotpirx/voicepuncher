//! A process that takes connections on a loopback port and never answers them: a hung service.

use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// Listens without answering, as a separate process whose command line ends in a given word,
/// so a test can tell a hung copy of a service from another program on its port.
pub struct SilentListener {
    child: Child,
    pub port: u16,
}

/// Listens on the port it is given, accepts nothing and sleeps.
const LISTEN: &str = "my $socket = IO::Socket::INET->new(LocalAddr => '127.0.0.1', LocalPort => $ARGV[0], Listen => 5, ReuseAddr => 1) or die $!; sleep 600";

impl SilentListener {
    /// On a free loopback port, its command line ending in `named`.
    pub fn start(named: &str) -> Self {
        Self::on(free_port(), named)
    }

    /// On `port`, its command line ending in `named`; returns once the port takes connections.
    pub fn on(port: u16, named: &str) -> Self {
        let child = Command::new("/usr/bin/perl")
            .args(["-MIO::Socket::INET", "-e", LISTEN, &port.to_string(), named])
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while TcpStream::connect(("127.0.0.1", port)).is_err() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        Self { child, port }
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Whether the process still runs.
    pub fn runs(&mut self) -> bool {
        self.child.try_wait().is_ok_and(|status| status.is_none())
    }
}

impl Drop for SilentListener {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A loopback port nothing listens on right now.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap()
}
