//! The HTTP plumbing and port reservation of the text refinement service stubs.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CapturedRequest {
    pub request_line: String,
    pub authorization: Option<String>,
    pub body: String,
}

pub fn read_request(stream: &mut TcpStream) -> Option<CapturedRequest> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let (mut content_length, mut authorization) = (0usize, None);
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        match name.to_ascii_lowercase().as_str() {
            "content-length" => content_length = value.trim().parse().ok()?,
            "authorization" => authorization = Some(value.trim().to_owned()),
            _ => {}
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).ok()?;
    Some(CapturedRequest {
        request_line: request_line.trim_end().to_owned(),
        authorization,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

pub fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let response = format!(
        "HTTP/1.1 {status} STUB\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

/// The service ports the fake copies' addresses take: their copies land 40 000 below, on
/// 2001–8004, clear of the system's ephemeral ports (49152+) and of the user's services
/// (8080–8085).
pub const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 42_000..48_000;
/// A port of `FAKE_SERVICE_PORTS` bound without listening for the life of the test process:
/// connections to it are refused, so the pool looks for copies.
pub fn reserve_port() -> u16 {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let span = FAKE_SERVICE_PORTS.len();
    let start = std::process::id() as usize * 7_919;
    loop {
        let index = (start + NEXT.fetch_add(1, Ordering::SeqCst)) % span;
        let port = FAKE_SERVICE_PORTS.start + index as u16;
        if bind_without_listening(port) {
            return port;
        }
    }
}

pub fn bind_without_listening(port: u16) -> bool {
    // SAFETY: a plain socket call with constant arguments.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    assert!(fd >= 0, "socket: {}", std::io::Error::last_os_error());
    // SAFETY: sockaddr_in is plain data, valid when zeroed.
    let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    let len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
    addr.sin_len = len as u8;
    addr.sin_family = libc::AF_INET as libc::sa_family_t;
    addr.sin_port = port.to_be();
    addr.sin_addr.s_addr = u32::from(std::net::Ipv4Addr::LOCALHOST).to_be();
    // SAFETY: `addr` is a sockaddr_in of length `len`, alive for the call; a bound descriptor
    // is never closed, so its port stays reserved until the process exits.
    let bound = unsafe { libc::bind(fd, (&raw const addr).cast(), len) } == 0;
    if !bound {
        // SAFETY: `fd` is this function's own open descriptor, closed once.
        unsafe { libc::close(fd) };
    }
    bound
}
