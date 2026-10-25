//! Stand-in services on free loopback ports, for the tests that talk to one.

use std::io::Read;
use std::net::{TcpListener, TcpStream};

/// A service on a free loopback port that hands every connection to `answer`, one after
/// another on a thread of its own; its port.
pub fn serve_each(answer: impl FnMut(TcpStream) + Send + 'static) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut answer = answer;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            answer(stream);
        }
    });
    port
}

/// Reads the start of the request on `stream`, at most 4 KiB, and drops it.
pub fn skip_request(stream: &mut TcpStream) {
    let mut request = [0u8; 4096];
    let _ = stream.read(&mut request);
}
