//! The loopback WebSocket services the command spotting client tests stand up.

use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use tungstenite::WebSocket;

/// How long a stand-in service waits on a read before it looks again.
const READ_WAIT: Duration = Duration::from_millis(10);

/// A listener on a free loopback port and the service URL it answers on.
pub fn free_listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (listener, url)
}

/// A service that accepts one connection and hands it to `serve` on a thread of its own, its
/// reads waiting at most [`READ_WAIT`]; the service's URL.
pub fn serve_one(serve: impl FnOnce(WebSocket<TcpStream>) + Send + 'static) -> String {
    let (listener, url) = free_listener();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let socket = tungstenite::accept(stream).unwrap();
        socket.get_ref().set_read_timeout(Some(READ_WAIT)).unwrap();
        serve(socket);
    });
    url
}

/// Whether a read failed only because nothing came within its wait.
pub fn nothing_yet(error: &tungstenite::Error) -> bool {
    matches!(error, tungstenite::Error::Io(e) if e.kind() == std::io::ErrorKind::WouldBlock)
}
