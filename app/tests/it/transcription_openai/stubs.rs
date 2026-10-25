//! Stand-in speech recognition services and the log capture the tests share.

use super::*;

pub(super) struct StubReply {
    pub(crate) status: u16,
    pub(crate) body: &'static str,
    pub(crate) delay: Duration,
}

impl StubReply {
    pub(crate) fn ok(body: &'static str) -> Self {
        Self { status: 200, body, delay: Duration::ZERO }
    }
}

pub(super) struct CapturedRequest {
    pub(crate) request_line: String,
    pub(crate) content_type: String,
    pub(crate) body: String,
}

/// Serves exactly one HTTP request; connections that close without sending
/// anything (the reachability probe) are skipped.
pub(super) fn spawn_stub(reply: StubReply) -> (TranscriptionCfg, mpsc::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Some(request) = read_request(&mut stream.unwrap(), &reply) else {
                continue;
            };
            let _ = tx.send(request);
            return;
        }
    });
    (cfg_for(port), rx)
}

pub(super) fn read_request(stream: &mut TcpStream, reply: &StubReply) -> Option<CapturedRequest> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let (content_length, content_type) = read_headers(&mut reader)?;
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).ok()?;
    thread::sleep(reply.delay);
    let response = format!(
        "HTTP/1.1 {} STUB\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        reply.status,
        reply.body.len(),
        reply.body
    );
    let _ = stream.write_all(response.as_bytes());
    Some(CapturedRequest {
        request_line: request_line.trim_end().to_owned(),
        content_type,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

/// The body's length and type from the headers of a request.
fn read_headers(reader: &mut impl std::io::BufRead) -> Option<(usize, String)> {
    let (mut content_length, mut content_type) = (0usize, String::new());
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            return Some((content_length, content_type));
        }
        let (name, value) = line.split_once(':')?;
        match name.to_ascii_lowercase().as_str() {
            "content-length" => content_length = value.trim().parse().ok()?,
            "content-type" => value.trim().clone_into(&mut content_type),
            _ => {}
        }
    }
}

/// Resets the first `drops` requests (closes them without reading the body, so the OS
/// sends RST, as a busy service does), then answers with `reply`. Counts requests;
/// the reachability probe sends nothing and is not counted.
pub(super) fn spawn_dropping_stub(
    drops: usize,
    reply: StubReply,
) -> (TranscriptionCfg, Arc<AtomicUsize>) {
    spawn_counting_stub(move |n, mut stream| {
        if n < drops {
            drop(stream);
        } else {
            read_request(&mut stream, &reply);
        }
    })
}

/// A stand-in service that hands each request to `answer` with its number, from 0; the
/// reachability probe sends nothing and is neither answered nor counted. Counts requests.
pub(super) fn spawn_counting_stub(
    answer: impl Fn(usize, TcpStream) + Send + 'static,
) -> (TranscriptionCfg, Arc<AtomicUsize>) {
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    let port = serve_each(move |stream| {
        let mut first = [0u8; 1];
        if stream.peek(&mut first).unwrap_or(0) > 0 {
            answer(counter.fetch_add(1, Ordering::SeqCst), stream);
        }
    });
    (cfg_for(port), attempts)
}

/// The transcription settings for a stand-in service on `port`.
pub(super) fn cfg_for(port: u16) -> TranscriptionCfg {
    TranscriptionCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..TranscriptionCfg::default()
    }
}

/// Answers every request with `body` on a connection it keeps open, as the real
/// service does. After the first answer it waits for a message on the returned
/// sender, resets that connection (as macOS reports a connection the service closed
/// minutes ago) and confirms on the returned receiver.
pub(super) fn spawn_resetting_stub(
    body: &'static str,
) -> (TranscriptionCfg, mpsc::Sender<()>, mpsc::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (reset_tx, reset_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reset_done = false;
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            if !reset_done {
                reset_done = reset_after_first_answer(stream, body, &reset_rx, &done_tx);
                continue;
            }
            while answer_kept_open(&mut stream, body) {}
        }
    });
    (cfg_for(port), reset_tx, done_rx)
}

/// Answers one request on `stream`, then waits for a message on `asked`, resets the
/// connection and confirms on `done`; false when the client closed the connection first.
fn reset_after_first_answer(
    mut stream: TcpStream,
    body: &str,
    asked: &mpsc::Receiver<()>,
    done: &mpsc::Sender<()>,
) -> bool {
    if !answer_kept_open(&mut stream, body) {
        return false;
    }
    asked.recv().unwrap();
    reset(stream);
    done.send(()).unwrap();
    true
}

/// Reads one request and answers it with `body`, leaving the connection open; false
/// when the client closed the connection instead.
pub(super) fn answer_kept_open(stream: &mut TcpStream, body: &str) -> bool {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut content_length = 0usize;
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return false;
    }
    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap();
            }
        }
    }
    reader.read_exact(&mut vec![0u8; content_length]).unwrap();
    let response = format!(
        "HTTP/1.1 200 STUB\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).unwrap();
    true
}

pub(super) use crate::resets::reset;

/// Keeps every log message with the thread that logged it, so parallel tests read
/// only their own messages.
pub(super) struct CapturingLogger(pub(super) Mutex<Vec<(ThreadId, String)>>);

impl log::Log for CapturingLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        let line = format!("{} {}", record.level(), record.args());
        self.0.lock().unwrap().push((thread::current().id(), line));
    }
    fn flush(&self) {}
}

pub(super) fn capturing_logger() -> &'static CapturingLogger {
    static LOGGER: OnceLock<&'static CapturingLogger> = OnceLock::new();
    LOGGER.get_or_init(|| {
        let logger = Box::leak(Box::new(CapturingLogger(Mutex::new(Vec::new()))));
        log::set_logger(logger).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
        logger
    })
}

/// The lines any thread logged that contain `fragment`: the pool logs from its own threads.
pub(super) fn logged_by_any_thread(fragment: &str) -> Vec<String> {
    let lines = capturing_logger().0.lock().unwrap();
    lines
        .iter()
        .filter(|(_, line)| line.contains(fragment))
        .map(|(_, line)| line.clone())
        .collect()
}

pub(super) fn logged_by_this_thread(fragment: &str) -> usize {
    let me = thread::current().id();
    capturing_logger()
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|(thread, line)| *thread == me && line.contains(fragment))
        .count()
}

pub(super) fn half_second() -> AudioBuffer16k {
    AudioBuffer16k::new(vec![100i16; 8_000])
}

pub(super) fn transcribe(cfg: TranscriptionCfg) -> anyhow::Result<Option<String>> {
    let mut transcriber = OpenAiTranscriber::connect(cfg)?.with_recovery_wait(SHORT_RECOVERY_WAIT);
    let transcript = transcriber.transcribe(&half_second())?;
    Ok(transcript.map(|t| t.as_str().to_owned()))
}
