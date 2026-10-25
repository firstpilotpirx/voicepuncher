//! A probe that asks the program on a port whether it is a whisper-server.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::holder::holder_of;
use super::{Identity, Probe, TcpProbe};

/// How long the program on a port has to answer what it is; a whisper-server answers its
/// page in milliseconds, also while it decodes.
const ANSWER_LIMIT: Duration = Duration::from_secs(1);
/// The start of the page is enough to find the name in it.
const PAGE_LIMIT: u64 = 64 * 1024;

/// Accepts a port as a copy of a speech recognition service only when `GET /` answers with a
/// page that names whisper, as the service manager's probe does.
pub struct WhisperProbe {
    tcp: TcpProbe,
}

impl WhisperProbe {
    pub fn new(host: &str) -> Self {
        Self { tcp: TcpProbe::new(host) }
    }

    fn page(&self, port: u16) -> Option<Vec<u8>> {
        let address = self.address(port)?;
        let mut stream = TcpStream::connect_timeout(&address, ANSWER_LIMIT).ok()?;
        stream.set_read_timeout(Some(ANSWER_LIMIT)).ok()?;
        stream.set_write_timeout(Some(ANSWER_LIMIT)).ok()?;
        let request = format!(
            "GET / HTTP/1.1\r\nHost: {}:{port}\r\nConnection: close\r\n\r\n",
            self.tcp.host
        );
        stream.write_all(request.as_bytes()).ok()?;
        let mut page = Vec::new();
        let read = stream.take(PAGE_LIMIT).read_to_end(&mut page);
        (read.is_ok() || !page.is_empty()).then_some(page)
    }

    fn address(&self, port: u16) -> Option<SocketAddr> {
        (self.tcp.host.as_str(), port)
            .to_socket_addrs()
            .ok()?
            .next()
    }
}

impl Probe for WhisperProbe {
    fn answers(&self, port: u16) -> bool {
        self.tcp.answers(port)
    }

    fn identify(&self, port: u16) -> Identity {
        match self.page(port) {
            None => Identity::Unknown,
            Some(page) if names_whisper(&page) => Identity::Service,
            Some(_) => Identity::Foreign(holder_of(port)),
        }
    }
}

/// Whether `page` is an HTTP/1 answer `200` whose text names whisper.
fn names_whisper(page: &[u8]) -> bool {
    let text = String::from_utf8_lossy(page).to_lowercase();
    let status = text.lines().next().unwrap_or_default();
    status.starts_with("http/1.")
        && status.split(' ').nth(1) == Some("200")
        && text.contains("whisper")
}

#[cfg(test)]
mod tests;
