//! A stand-in speech recognition service on loopback: it takes the requests in the order they
//! come, across all its copies, as its script says (the last turn repeats), and keeps every
//! request it took with the copy that took it.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_copy_pool::ServiceAddress;

/// The text the stand-in service hears in every recording it answers.
pub const HEARD: &str = "Meeting moved to Friday.";

/// What the service does with one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    /// Answers with this status and JSON body.
    Answer(u16, &'static str),
    /// Reads the request and never answers it.
    Hang,
    /// Closes the connection with the request unread, so the system resets it, as a busy or
    /// killed service does.
    Drop,
}

/// The answer that carries [`HEARD`].
pub const HEARS: Turn = Turn::Answer(200, r#"{"text": "Meeting moved to Friday."}"#);
/// The answer of a service that heard no speech.
pub const HEARS_NOTHING: Turn = Turn::Answer(200, r#"{"text": ""}"#);
/// The answer of a service too busy to take the request.
pub const ERROR: Turn = Turn::Answer(503, r#"{"error": "busy"}"#);

/// How long a hung request holds its connection: longer than any scenario waits.
const HOLD: Duration = Duration::from_secs(60);

/// One request the service took.
#[derive(Debug, Clone)]
pub struct Taken {
    /// The port of the copy that took it.
    pub port: u16,
    /// The request body; empty for a dropped request, which the service never read.
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Script {
    turns: Arc<Vec<Turn>>,
    next: Arc<AtomicUsize>,
    taken: Arc<Mutex<Vec<Taken>>>,
}

/// The stand-in service and what it took.
#[derive(Debug)]
pub struct FakeService {
    pub base_url: String,
    script: Script,
}

impl FakeService {
    /// One service at its own address, without copies.
    pub fn single(turns: Vec<Turn>) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let base_url = format!("http://127.0.0.1:{}/v1", listener.local_addr()?.port());
        Ok(Self::serving(base_url, vec![listener], turns))
    }

    /// `n` copies at the ports the copy pool finds from a base address nothing listens on.
    pub fn copies(n: usize, turns: Vec<Turn>) -> Result<Self> {
        for _ in 0..100 {
            let base = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
            let base_url = format!("http://127.0.0.1:{base}/v1");
            let ports = ServiceAddress::parse(&base_url)?.copy_ports();
            let bound: std::io::Result<Vec<_>> = ports[..n]
                .iter()
                .map(|port| TcpListener::bind(("127.0.0.1", *port)))
                .collect();
            if let Ok(listeners) = bound {
                return Ok(Self::serving(base_url, listeners, turns));
            }
        }
        anyhow::bail!("no free ports for {n} copies")
    }

    fn serving(base_url: String, listeners: Vec<TcpListener>, turns: Vec<Turn>) -> Self {
        let script = Script { turns: Arc::new(turns), next: Arc::default(), taken: Arc::default() };
        for listener in listeners {
            let script = script.clone();
            std::thread::spawn(move || accept(&listener, &script));
        }
        Self { base_url, script }
    }

    /// Every request taken so far, in the order the service took them.
    pub fn taken(&self) -> Vec<Taken> {
        self.script.taken.lock().unwrap().clone()
    }
}

fn accept(listener: &TcpListener, script: &Script) {
    let Ok(port) = listener.local_addr().map(|a| a.port()) else {
        return;
    };
    for stream in listener.incoming().flatten() {
        let script = script.clone();
        std::thread::spawn(move || serve(stream, port, &script));
    }
}

/// Takes one request as the script says; a connection that sends nothing (the pool's probe)
/// is no request.
fn serve(mut stream: TcpStream, port: u16, script: &Script) {
    if !voicepuncher_test_steps::loopback::is_request(&mut stream) {
        return;
    }
    let index = script.next.fetch_add(1, Ordering::SeqCst);
    let turn = script.turns[index.min(script.turns.len() - 1)];
    if turn == Turn::Drop {
        script
            .taken
            .lock()
            .unwrap()
            .push(Taken { port, body: Vec::new() });
        return;
    }
    let Ok(body) = request_body(&mut stream) else {
        return;
    };
    script.taken.lock().unwrap().push(Taken { port, body });
    match turn {
        Turn::Answer(status, json) => {
            let reply = format!(
                "HTTP/1.1 {status} FAKE\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{json}",
                json.len()
            );
            let _ = stream.write_all(reply.as_bytes());
        }
        Turn::Hang => std::thread::sleep(HOLD),
        Turn::Drop => {}
    }
}

/// Reads the head of an HTTP request byte by byte, then as many body bytes as it announces.
fn request_body(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte)?;
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
    let length = head
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .context("no content length")?
        .trim()
        .parse()?;
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body)?;
    Ok(body)
}
