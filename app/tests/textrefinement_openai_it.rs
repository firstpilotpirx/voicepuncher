// Integration tests for OpenAiTextRefiner against an in-test HTTP stub.
// One test is gated on VOICEPUNCHER_REFINEMENT_URL and refines a sentence through a
// real text refinement service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, ThreadId};
use std::time::Duration;

use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::config::RefinementCfg;
use voicepuncher::core::application::TextRefiner;
use voicepuncher::core::domain::RawTranscript;

const MODELS: &str = r#"{"object":"list","data":[{"id":"qwen"}]}"#;
const TRANSCRIPT: &str = "the docker image builds fine on staging";
const ANSWER: &str = r#"{"choices":[{"message":{"role":"assistant","content":"The Docker image builds fine on staging."},"finish_reason":"stop"}]}"#;
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

struct StubReply {
    status: u16,
    body: &'static str,
    delay: Duration,
}

impl StubReply {
    fn ok(body: &'static str) -> Self {
        Self {
            status: 200,
            body,
            delay: Duration::ZERO,
        }
    }
}

struct CapturedRequest {
    request_line: String,
    authorization: Option<String>,
    body: String,
}

/// Answers the startup model list, then serves exactly one chat request with `reply`.
fn spawn_stub(reply: StubReply) -> (RefinementCfg, mpsc::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let Some(request) = read_request(&mut stream) else {
                continue;
            };
            if request.request_line.starts_with("GET ") {
                respond(&mut stream, 200, MODELS);
                continue;
            }
            thread::sleep(reply.delay);
            respond(&mut stream, reply.status, reply.body);
            let _ = tx.send(request);
            return;
        }
    });
    let cfg = RefinementCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..RefinementCfg::default()
    };
    (cfg, rx)
}

fn read_request(stream: &mut TcpStream) -> Option<CapturedRequest> {
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

fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let response = format!(
        "HTTP/1.1 {status} STUB\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

/// Answers the startup model list, then every chat request with `body` on a
/// connection it keeps open, as the real service does. After the first chat answer it
/// waits for a message on the returned sender, resets that connection (as macOS
/// reports a connection the service closed minutes ago) and confirms on the returned
/// receiver.
fn spawn_resetting_stub(
    body: &'static str,
) -> (RefinementCfg, mpsc::Sender<()>, mpsc::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (reset_tx, reset_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reset_done = false;
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            while let Some(request) = read_request(&mut stream) {
                if request.request_line.starts_with("GET ") {
                    respond(&mut stream, 200, MODELS);
                    break;
                }
                respond_kept_open(&mut stream, body);
                if !reset_done {
                    reset_rx.recv().unwrap();
                    reset(stream);
                    reset_done = true;
                    done_tx.send(()).unwrap();
                    break;
                }
            }
        }
    });
    let cfg = RefinementCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..RefinementCfg::default()
    };
    (cfg, reset_tx, done_rx)
}

fn respond_kept_open(stream: &mut TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 STUB\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).unwrap();
}

/// Closes the connection with a reset instead of an orderly close.
fn reset(stream: TcpStream) {
    let linger = libc::linger {
        l_onoff: 1,
        l_linger: 0,
    };
    // SAFETY: `stream` owns a valid socket; `linger` is a valid SO_LINGER value of its size.
    let rc = unsafe {
        libc::setsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            (&linger as *const libc::linger).cast(),
            std::mem::size_of::<libc::linger>() as libc::socklen_t,
        )
    };
    assert_eq!(rc, 0, "set SO_LINGER");
    drop(stream);
}

fn refine(cfg: RefinementCfg, transcript: &str) -> anyhow::Result<String> {
    let mut refiner = OpenAiTextRefiner::connect(cfg)?;
    let cleaned = refiner.refine(&RawTranscript::new(transcript.to_owned())?)?;
    Ok(cleaned.as_str().to_owned())
}

#[test]
fn should_post_chat_request_and_return_cleaned_text() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(ANSWER));
    let cfg = RefinementCfg {
        model: "qwen".into(),
        ..cfg
    };
    // When
    let cleaned = refine(cfg, TRANSCRIPT).unwrap();
    // Then
    assert_eq!(cleaned, "The Docker image builds fine on staging.");
    let request = requests.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(request.request_line, "POST /v1/chat/completions HTTP/1.1");
    let body: serde_json::Value = serde_json::from_str(&request.body).unwrap();
    assert_eq!(body["model"], "qwen");
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.first().unwrap()["role"], "system");
    assert_eq!(
        messages.last().unwrap()["content"],
        format!("<transcript>\n{TRANSCRIPT}\n</transcript>")
    );
    assert_eq!(request.authorization, None);
}

#[test]
fn should_send_api_key_as_bearer_token() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(ANSWER));
    let cfg = RefinementCfg {
        api_key: "secret".into(),
        ..cfg
    };
    // When
    refine(cfg, TRANSCRIPT).unwrap();
    // Then
    let request = requests.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(request.authorization.as_deref(), Some("Bearer secret"));
}

#[test]
fn should_fail_on_server_error() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply {
        status: 500,
        ..StubReply::ok(r#"{"error": "boom"}"#)
    });
    // When / Then
    assert!(refine(cfg, TRANSCRIPT).is_err());
}

#[test]
fn should_fail_on_malformed_json() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok("not json"));
    // When / Then
    let message = format!("{:#}", refine(cfg, TRANSCRIPT).unwrap_err());
    assert!(
        message.contains("unexpected text refinement response"),
        "{message}"
    );
}

#[test]
fn should_fail_when_service_answers_after_timeout() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply {
        delay: Duration::from_secs(3),
        ..StubReply::ok(ANSWER)
    });
    let cfg = RefinementCfg {
        timeout_secs: 1,
        ..cfg
    };
    // When
    let started = std::time::Instant::now();
    let result = refine(cfg, TRANSCRIPT);
    // Then it fails after one wait: a timeout is not retried, the raw transcript is good enough
    assert!(result.is_err());
    assert!(
        started.elapsed() < Duration::from_millis(1_800),
        "waited {:?}: the timed-out request was sent again",
        started.elapsed()
    );
}

#[test]
fn should_refine_after_service_reset_idle_connection() {
    // Given
    let (cfg, reset, reset_done) = spawn_resetting_stub(ANSWER);
    let mut refiner = OpenAiTextRefiner::connect(cfg).unwrap();
    let transcript = RawTranscript::new(TRANSCRIPT.to_owned()).unwrap();
    let first = refiner.refine(&transcript).unwrap();
    reset.send(()).unwrap();
    reset_done.recv_timeout(RECV_TIMEOUT).unwrap();
    thread::sleep(Duration::from_millis(50));
    // When
    let next = refiner.refine(&transcript);
    // Then
    assert_eq!(first.as_str(), "The Docker image builds fine on staging.");
    assert_eq!(
        next.unwrap().as_str(),
        "The Docker image builds fine on staging."
    );
}

/// Keeps every log message with the thread that logged it, so parallel tests read
/// only their own messages.
struct CapturingLogger(Mutex<Vec<(ThreadId, String)>>);

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

fn capturing_logger() -> &'static CapturingLogger {
    static LOGGER: OnceLock<&'static CapturingLogger> = OnceLock::new();
    LOGGER.get_or_init(|| {
        let logger = Box::leak(Box::new(CapturingLogger(Mutex::new(Vec::new()))));
        log::set_logger(logger).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
        logger
    })
}

fn logged_by_this_thread(fragment: &str) -> usize {
    let me = thread::current().id();
    capturing_logger()
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|(thread, line)| *thread == me && line.contains(fragment))
        .count()
}

/// Answers the startup model list, then resets the first `drops` chat requests (as a busy
/// service does) and answers the later ones with `ANSWER`. Counts chat requests.
fn spawn_dropping_stub(drops: usize) -> (RefinementCfg, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let posts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&posts);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let Some(request) = read_request(&mut stream) else {
                continue;
            };
            if request.request_line.starts_with("GET ") {
                respond(&mut stream, 200, MODELS);
            } else if counter.fetch_add(1, Ordering::SeqCst) < drops {
                reset(stream);
            } else {
                respond(&mut stream, 200, ANSWER);
            }
        }
    });
    let cfg = RefinementCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..RefinementCfg::default()
    };
    (cfg, posts)
}

#[test]
fn should_refine_by_a_new_attempt_after_one_dropped_connection() {
    // Given
    capturing_logger();
    let (cfg, posts) = spawn_dropping_stub(1);
    // When
    let cleaned = refine(cfg, TRANSCRIPT).unwrap();
    // Then
    assert_eq!(cleaned, "The Docker image builds fine on staging.");
    assert_eq!(posts.load(Ordering::SeqCst), 2);
    assert_eq!(
        logged_by_this_thread("WARN text refinement service dropped the connection"),
        1
    );
}

#[test]
fn should_send_the_request_twice_when_the_service_keeps_dropping_the_connection() {
    // Given
    let (cfg, posts) = spawn_dropping_stub(usize::MAX);
    // When
    let result = refine(cfg, TRANSCRIPT);
    // Then the refiner gives up, and the raw transcript is pasted instead
    assert!(result.is_err());
    assert_eq!(posts.load(Ordering::SeqCst), 2);
}
