//! How a failed request to a local service is told apart, on real loopback sockets: a
//! service that never answers, one that resets the connection, one that answers busy.

use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::time::Duration;

use voicepuncher::adapters::secondary::http::failure::Failure;

const DEADLINE: Duration = Duration::from_millis(300);

/// A service that handles each connection with `handle`; returns its address.
fn stub(handle: fn(TcpStream)) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || handle(stream));
        }
    });
    url
}

fn failure_of(url: &str) -> Failure {
    let error = ureq::post(url)
        .timeout(DEADLINE)
        .send_string("audio")
        .unwrap_err();
    Failure::of(&error)
}

fn read_some(stream: &mut TcpStream) {
    let mut buf = [0u8; 4096];
    let _ = stream.read(&mut buf);
}

#[test]
fn should_tell_a_service_that_never_answers_as_timed_out() {
    let url = stub(|mut stream| {
        read_some(&mut stream);
        std::thread::sleep(Duration::from_secs(5));
    });
    assert_eq!(failure_of(&url), Failure::TimedOut);
}

#[test]
fn should_tell_a_reset_connection_as_dropped() {
    let url = stub(|mut stream| {
        read_some(&mut stream);
        let linger = libc::linger { l_onoff: 1, l_linger: 0 };
        // SAFETY: the descriptor is open for the stream's lifetime and the option is a plain struct.
        unsafe {
            libc::setsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_LINGER,
                (&raw const linger).cast(),
                std::mem::size_of::<libc::linger>() as libc::socklen_t,
            );
        }
    });
    assert_eq!(failure_of(&url), Failure::Dropped);
}

/// Reads the whole request, up to its body `audio`: a stub that answers and closes with part
/// of the request unread makes the kernel reset the connection, and the client sees no answer.
fn read_request(stream: &mut TcpStream) {
    let mut request = Vec::new();
    let mut buf = [0u8; 4096];
    while !request.ends_with(b"audio") {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&buf[..n]),
        }
    }
}

#[test]
fn should_tell_a_busy_answer_as_busy() {
    let url = stub(|mut stream| {
        read_request(&mut stream);
        let _ = std::io::Write::write_all(
            &mut stream,
            b"HTTP/1.1 503 Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
    });
    assert_eq!(failure_of(&url), Failure::Busy(503));
}

#[test]
fn should_tell_a_port_nobody_listens_on_as_dropped() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    assert_eq!(
        failure_of(&format!("http://127.0.0.1:{port}/v1")),
        Failure::Dropped
    );
}
