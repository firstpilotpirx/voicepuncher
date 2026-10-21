//! The model fetcher against a local HTTP server: a full download, a resume with a Range
//! request, a server that ignores the range, a redirect that keeps the range, a refused port.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use voicepuncher::adapters::secondary::modelfetch::http::HttpModelFetcher;
use voicepuncher::core::application::{FetchError, ModelFetcher, PartSink};

const BODY: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// How the server answers a request for `/file`.
#[derive(Clone, Copy)]
enum Mode {
    HonourRange,
    IgnoreRange,
    /// `/file` redirects to `/moved`, which honours the range.
    Redirect,
}

/// A server on a free port, serving `BODY`; the `Range` header of every request it saw.
struct Server {
    port: u16,
    ranges: Arc<Mutex<Vec<Option<String>>>>,
}

impl Server {
    fn start(mode: Mode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let ranges = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&ranges);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                serve(stream, mode, &seen);
            }
        });
        Self { port, ranges }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/file", self.port)
    }

    fn ranges(&self) -> Vec<Option<String>> {
        self.ranges.lock().unwrap().clone()
    }
}

fn serve(mut stream: TcpStream, mode: Mode, seen: &Mutex<Vec<Option<String>>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let mut range = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("range") {
                range = Some(value.trim().to_string());
            }
        }
    }
    seen.lock().unwrap().push(range.clone());
    if matches!(mode, Mode::Redirect) && path == "/file" {
        let _ = write!(
            stream,
            "HTTP/1.1 302 Found\r\nLocation: /moved\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        return;
    }
    let from = range
        .as_deref()
        .and_then(|r| r.strip_prefix("bytes="))
        .and_then(|r| r.trim_end_matches('-').parse::<usize>().ok());
    match (mode, from) {
        (Mode::IgnoreRange, _) | (_, None) => {
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                BODY.len()
            );
            let _ = stream.write_all(BODY);
        }
        (_, Some(from)) => {
            let rest = &BODY[from..];
            let _ = write!(
                stream,
                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                BODY.len() - 1,
                BODY.len(),
                rest.len()
            );
            let _ = stream.write_all(rest);
        }
    }
}

/// A part in memory.
#[derive(Default)]
struct Part(Vec<u8>);

impl PartSink for Part {
    fn write(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.0.extend_from_slice(bytes);
        Ok(())
    }

    fn restart(&mut self) -> Result<(), String> {
        self.0.clear();
        Ok(())
    }

    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn sha256_hex(&mut self) -> String {
        String::new()
    }
}

fn fetch(url: &str, part: &mut Part) -> (Result<(), FetchError>, Vec<u64>) {
    let mut received = Vec::new();
    let result = HttpModelFetcher::new().fetch(
        url,
        part,
        &mut |n| received.push(n),
        &AtomicBool::new(false),
    );
    (result, received)
}

#[test]
fn should_download_the_whole_file_into_an_empty_part() {
    // Given
    let server = Server::start(Mode::HonourRange);
    let mut part = Part::default();
    // When
    let (result, received) = fetch(&server.url(), &mut part);
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(part.0, BODY);
    assert_eq!(received.last(), Some(&(BODY.len() as u64)));
    assert_eq!(server.ranges(), vec![None]);
}

#[test]
fn should_ask_only_for_the_rest_of_a_part() {
    // Given
    let server = Server::start(Mode::HonourRange);
    let mut part = Part(BODY[..10].to_vec());
    // When
    let (result, _) = fetch(&server.url(), &mut part);
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(part.0, BODY);
    assert_eq!(server.ranges(), vec![Some("bytes=10-".to_string())]);
}

#[test]
fn should_start_over_when_the_server_sends_the_whole_file_to_a_resume() {
    // Given
    let server = Server::start(Mode::IgnoreRange);
    let mut part = Part(b"stale".to_vec());
    // When
    let (result, _) = fetch(&server.url(), &mut part);
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(part.0, BODY);
}

#[test]
fn should_keep_the_range_across_a_redirect() {
    // Given
    let server = Server::start(Mode::Redirect);
    let mut part = Part(BODY[..20].to_vec());
    // When
    let (result, _) = fetch(&server.url(), &mut part);
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(part.0, BODY);
    assert_eq!(
        server.ranges(),
        vec![Some("bytes=20-".to_string()), Some("bytes=20-".to_string())]
    );
}

#[test]
fn should_take_a_refused_port_for_a_connection_failure() {
    // Given
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut part = Part::default();
    // When
    let (result, _) = fetch(&format!("http://127.0.0.1:{port}/file"), &mut part);
    // Then
    assert!(
        matches!(result, Err(FetchError::Connection(_))),
        "{result:?}"
    );
}

#[test]
fn should_stop_at_once_when_cancelled() {
    // Given
    let server = Server::start(Mode::HonourRange);
    let mut part = Part::default();
    // When
    let result = HttpModelFetcher::new().fetch(
        &server.url(),
        &mut part,
        &mut |_| {},
        &AtomicBool::new(true),
    );
    // Then
    assert_eq!(result, Err(FetchError::Cancelled));
}
