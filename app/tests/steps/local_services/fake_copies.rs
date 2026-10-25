//! Fake copies of a speech recognition service on the copy ports of an address nothing else
//! listens on: each answers every request with a transcript after a while, and counts the
//! requests it got and how many it held at once.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher_copy_pool::ServiceAddress;

/// How long a fake copy works on a request.
pub(super) const COPY_WORK: Duration = Duration::from_millis(200);
/// The service ports the fake copies' addresses take: their copies land clear of the
/// system's ephemeral ports and of the user's and the test services' ports.
const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 11_000..18_000;
const ANSWER: &str = r#"{"text": "hello"}"#;

/// A fake copy: the requests it got, and the most it held at once.
#[derive(Debug, Clone)]
pub(super) struct FakeCopy {
    pub(super) port: u16,
    requests: Arc<AtomicUsize>,
    most_at_once: Arc<AtomicUsize>,
}

impl FakeCopy {
    pub(super) fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    pub(super) fn most_at_once(&self) -> usize {
        self.most_at_once.load(Ordering::SeqCst)
    }
}

/// The address of a service whose own port is free, and listeners on its first `n` copy
/// ports; each scenario takes an address of its own, as the copies of one address are shared
/// by the whole process.
pub(super) fn free_copies(n: usize) -> anyhow::Result<(String, Vec<TcpListener>)> {
    let span = FAKE_SERVICE_PORTS.len();
    let start = std::process::id() as usize * 7_919;
    for step in 0..span {
        let Ok(offset) = u16::try_from((start + step * 13) % span) else {
            continue;
        };
        let port = FAKE_SERVICE_PORTS.start + offset;
        let base_url = format!("http://127.0.0.1:{port}/v1");
        if TcpListener::bind(("127.0.0.1", port)).is_err() {
            continue;
        }
        let Ok(address) = ServiceAddress::parse(&base_url) else {
            continue;
        };
        let listeners: Result<Vec<_>, _> = address.copy_ports()[..n]
            .iter()
            .map(|copy| TcpListener::bind(("127.0.0.1", *copy)))
            .collect();
        if let Ok(listeners) = listeners {
            return Ok((base_url, listeners));
        }
    }
    anyhow::bail!("no free service address in {FAKE_SERVICE_PORTS:?}")
}

/// The index of the copy this process tries first among `n` copies.
pub(super) fn first_tried(n: usize) -> usize {
    std::process::id() as usize % n
}

/// Serves every request on `listener` on a thread of its own; connections that send nothing
/// (the copies' probes) are not counted.
pub(super) fn serve(listener: TcpListener) -> FakeCopy {
    let port = listener.local_addr().map_or(0, |address| address.port());
    let copy = FakeCopy { port, requests: Arc::default(), most_at_once: Arc::default() };
    let in_flight = Arc::new(AtomicUsize::new(0));
    let served = copy.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (copy, in_flight) = (served.clone(), Arc::clone(&in_flight));
            std::thread::spawn(move || answer(stream, &copy, &in_flight));
        }
    });
    copy
}

fn answer(mut stream: TcpStream, copy: &FakeCopy, in_flight: &AtomicUsize) {
    if !voicepuncher_test_steps::loopback::is_request(&mut stream) {
        return;
    }
    copy.requests.fetch_add(1, Ordering::SeqCst);
    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
    copy.most_at_once.fetch_max(now, Ordering::SeqCst);
    if read_request(&stream).is_some() {
        std::thread::sleep(COPY_WORK);
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{ANSWER}",
            ANSWER.len()
        );
    }
    in_flight.fetch_sub(1, Ordering::SeqCst);
}

/// Reads one request whole: its head, then as many body bytes as it names.
fn read_request(stream: &TcpStream) -> Option<()> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().ok()?;
            }
        }
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()
}
