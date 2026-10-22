//! The health probe on the WebSocket port: told apart from a WebSocket client by its request
//! line, answered from the connections' health.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::health::Health;

/// How long the accept thread waits for a health probe's request line.
const PROBE_WAIT: Duration = Duration::from_secs(2);
const HEALTH_REQUEST: &[u8] = b"GET /health";
/// How long a peek waits before it looks again for the rest of the request line.
const PEEK_AGAIN: Duration = Duration::from_millis(10);

/// A plain HTTP `GET /health` on the WebSocket port, told apart by its request line.
pub(super) fn is_health_probe(stream: &TcpStream) -> bool {
    let mut line = [0u8; HEALTH_REQUEST.len()];
    let _ = stream.set_read_timeout(Some(PROBE_WAIT));
    let started = Instant::now();
    while started.elapsed() < PROBE_WAIT {
        match stream.peek(&mut line) {
            Ok(n) if n >= line.len() => return line == HEALTH_REQUEST,
            Ok(n) if n > 0 && !HEALTH_REQUEST.starts_with(&line[..n]) => return false,
            Ok(0) | Err(_) => return false,
            Ok(_) => std::thread::sleep(PEEK_AGAIN),
        }
    }
    false
}

pub(super) fn answer_health(mut stream: TcpStream, health: &Health) -> Result<()> {
    let mut request = [0u8; 1024];
    let _ = stream.read(&mut request);
    let (status, body) = match health.stuck() {
        None => ("200 OK", "ok\n".to_owned()),
        Some(conn) => {
            log::warn!("health: conn={conn} has audio waiting and made no progress");
            ("503 Service Unavailable", format!("stuck conn={conn}\n"))
        }
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}
