//! Stand-in services on free loopback ports, for the tests that talk to one.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

mod http2_only;
mod silent_listener;
pub use http2_only::Http2Only;
pub use silent_listener::{free_port, SilentListener};

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

/// The page a whisper-server answers `GET /` with, the start of it that names it.
const WHISPER_PAGE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 33\r\nConnection: close\r\n\r\n<title>Whisper.cpp Server</title>";

/// Answers as a whisper-server when the request on `stream` asks `GET /`, and tells whether it did.
///
/// The copy pool asks so once before a port joins: a stand-in copy of a speech recognition
/// service then drops the connection without counting it as a request. Leaves any other
/// request unread.
pub fn answered_as_whisper(stream: &mut TcpStream) -> bool {
    let mut start = [0u8; 6];
    let asks_page = stream
        .peek(&mut start)
        .is_ok_and(|peeked| start[..peeked].starts_with(b"GET / "));
    if asks_page {
        skip_request(stream);
        let _ = stream.write_all(WHISPER_PAGE.as_bytes());
    }
    asks_page
}

/// Whether `stream` carries a request for a stand-in copy: not the copy pool's probe, which
/// sends nothing, nor its `GET /`, answered here as a whisper-server.
pub fn is_request(stream: &mut TcpStream) -> bool {
    let mut first = [0u8; 1];
    !answered_as_whisper(stream) && stream.peek(&mut first).unwrap_or(0) > 0
}

/// A stand-in copy that only answers the copy pool's `GET /` as a whisper-server, until it
/// is closed: then its port refuses connections, as a copy that died.
#[derive(Debug)]
pub struct WhisperUntilClosed {
    open: std::sync::Arc<std::sync::atomic::AtomicBool>,
    serving: Option<std::thread::JoinHandle<()>>,
}

impl WhisperUntilClosed {
    pub fn on(listener: TcpListener) -> Self {
        use std::sync::atomic::{AtomicBool, Ordering};
        let open = std::sync::Arc::new(AtomicBool::new(true));
        let serving_open = std::sync::Arc::clone(&open);
        listener.set_nonblocking(true).unwrap();
        let serving = std::thread::spawn(move || {
            while serving_open.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => answer_blocking(&mut stream),
                    Err(_) => std::thread::sleep(std::time::Duration::from_millis(5)),
                }
            }
        });
        Self { open, serving: Some(serving) }
    }

    /// Closes the port and waits until it refuses connections.
    pub fn close(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.open.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(serving) = self.serving.take() {
            let _ = serving.join();
        }
    }
}

impl Drop for WhisperUntilClosed {
    fn drop(&mut self) {
        self.stop();
    }
}

fn answer_blocking(stream: &mut TcpStream) {
    if stream.set_nonblocking(false).is_ok() {
        answered_as_whisper(stream);
    }
}
