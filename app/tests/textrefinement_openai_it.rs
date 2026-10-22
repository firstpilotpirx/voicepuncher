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
use voicepuncher_copy_pool::{ServiceAddress, TEXT_REFINEMENT_COPY_OFFSET};

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
        Self { status: 200, body, delay: Duration::ZERO }
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
#[expect(clippy::excessive_nesting, reason = "size baseline")]
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
    let linger = libc::linger { l_onoff: 1, l_linger: 0 };
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
    let cfg = RefinementCfg { model: "qwen".into(), ..cfg };
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
    let cfg = RefinementCfg { api_key: "secret".into(), ..cfg };
    // When
    refine(cfg, TRANSCRIPT).unwrap();
    // Then
    let request = requests.recv_timeout(RECV_TIMEOUT).unwrap();
    assert_eq!(request.authorization.as_deref(), Some("Bearer secret"));
}

#[test]
fn should_fail_on_server_error() {
    // Given
    let (cfg, _requests) =
        spawn_stub(StubReply { status: 500, ..StubReply::ok(r#"{"error": "boom"}"#) });
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
    let (cfg, _requests) =
        spawn_stub(StubReply { delay: Duration::from_secs(3), ..StubReply::ok(ANSWER) });
    let cfg = RefinementCfg { timeout_secs: 1, ..cfg };
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
    assert_eq!(logged_by_this_thread(WAITING_FOR_A_COPY), 0);
}

#[test]
fn should_wait_30_seconds_for_a_copy_to_come_back_by_default() {
    assert_eq!(
        voicepuncher::adapters::secondary::textrefinement::openai::RECOVERY_WAIT,
        Duration::from_secs(30)
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

/// The service ports the fake copies' addresses take: their copies land 40 000 below, on
/// 2001–8004, clear of the system's ephemeral ports (49152+) and of the user's services
/// (8080–8085).
const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 42_000..48_000;
const SHORT_RECOVERY_WAIT: Duration = Duration::from_secs(5);
const COMES_BACK_AFTER: Duration = Duration::from_secs(3);
const WAITING_FOR_A_COPY: &str = "no copy accepts connections — waiting";

/// What a fake copy does with one chat request: the copies share the count of chat
/// requests, so a test decides by the request's turn, whichever copy got it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyTurn {
    Answer,
    Reset,
    /// Answers after this long.
    Silent(Duration),
}

/// Chat requests every fake copy of one service got, in order, by the copy's port.
type Posts = Arc<Mutex<Vec<u16>>>;

/// Listeners on the first `n` copy ports of a refinement service address nothing listens on.
fn free_copies(n: usize) -> (RefinementCfg, Vec<TcpListener>) {
    loop {
        let base = reserve_port();
        let base_url = format!("http://127.0.0.1:{base}/v1");
        let ports = ServiceAddress::parse(&base_url)
            .unwrap()
            .with_copy_offset(TEXT_REFINEMENT_COPY_OFFSET)
            .copy_ports();
        let listeners: Result<Vec<_>, _> = ports[..n]
            .iter()
            .map(|port| TcpListener::bind(("127.0.0.1", *port)))
            .collect();
        if let Ok(listeners) = listeners {
            let cfg = RefinementCfg { base_url, ..RefinementCfg::default() };
            return (cfg, listeners);
        }
    }
}

/// A port of `FAKE_SERVICE_PORTS` bound without listening for the life of the test process:
/// connections to it are refused, so the pool looks for copies.
fn reserve_port() -> u16 {
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

fn bind_without_listening(port: u16) -> bool {
    // SAFETY: plain socket calls on a zeroed sockaddr_in of the right length; a bound
    // descriptor is never closed, so its port stays reserved until the process exits.
    unsafe {
        let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(fd >= 0, "socket: {}", std::io::Error::last_os_error());
        let mut addr: libc::sockaddr_in = std::mem::zeroed();
        let len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        addr.sin_len = len as u8;
        addr.sin_family = libc::AF_INET as libc::sa_family_t;
        addr.sin_port = port.to_be();
        addr.sin_addr.s_addr = u32::from(std::net::Ipv4Addr::LOCALHOST).to_be();
        let bound = libc::bind(fd, (&raw const addr).cast(), len) == 0;
        if !bound {
            libc::close(fd);
        }
        bound
    }
}

/// Serves the model list and each chat request on a thread of its own, as `script` says
/// for the request's turn among every copy's chat requests; the last turn repeats.
#[expect(clippy::excessive_nesting, reason = "size baseline")]
fn spawn_copy(listener: TcpListener, script: Arc<Vec<CopyTurn>>, posts: Posts) {
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let (script, posts) = (Arc::clone(&script), Arc::clone(&posts));
            thread::spawn(move || {
                let Some(request) = read_request(&mut stream) else {
                    return;
                };
                if request.request_line.starts_with("GET ") {
                    respond(&mut stream, 200, MODELS);
                    return;
                }
                let turn = {
                    let mut posts = posts.lock().unwrap();
                    posts.push(port);
                    script[(posts.len() - 1).min(script.len() - 1)]
                };
                match turn {
                    CopyTurn::Answer => respond(&mut stream, 200, ANSWER),
                    CopyTurn::Reset => reset(stream),
                    CopyTurn::Silent(after) => {
                        thread::sleep(after);
                        respond(&mut stream, 200, ANSWER);
                    }
                }
            });
        }
    });
}

fn copies_with(script: Vec<CopyTurn>) -> (RefinementCfg, Posts) {
    let (cfg, listeners) = free_copies(2);
    let (script, posts) = (Arc::new(script), Posts::default());
    for listener in listeners {
        spawn_copy(listener, Arc::clone(&script), Arc::clone(&posts));
    }
    (cfg, posts)
}

#[test]
fn should_refine_on_the_other_copy_when_a_copy_is_killed_during_a_refinement() {
    // Given: the copy that gets the first request is killed while it refines
    capturing_logger();
    let (cfg, posts) = copies_with(vec![CopyTurn::Reset, CopyTurn::Answer]);
    // When
    let cleaned = refine(cfg, TRANSCRIPT).unwrap();
    // Then
    assert_eq!(cleaned, "The Docker image builds fine on staging.");
    let posts = posts.lock().unwrap().clone();
    assert_eq!(posts.len(), 2, "{posts:?}");
    assert_ne!(posts[0], posts[1], "the retry went to the same copy");
    assert_eq!(
        logged_by_this_thread("WARN text refinement service dropped the connection"),
        1
    );
}

#[test]
fn should_refine_on_another_copy_when_a_copy_sends_no_answer() {
    // Given: the copy that gets the first request answers only after the time limit
    let (cfg, posts) = copies_with(vec![
        CopyTurn::Silent(Duration::from_secs(4)),
        CopyTurn::Answer,
    ]);
    let cfg = RefinementCfg { timeout_secs: 1, ..cfg };
    // When
    let started = std::time::Instant::now();
    let cleaned = refine(cfg, TRANSCRIPT).unwrap();
    // Then
    assert_eq!(cleaned, "The Docker image builds fine on staging.");
    assert!(
        started.elapsed() < Duration::from_millis(2_500),
        "waited {:?}",
        started.elapsed()
    );
    let posts = posts.lock().unwrap().clone();
    assert_eq!(posts.len(), 2, "{posts:?}");
    assert_ne!(posts[0], posts[1], "the retry went to the same copy");
}

#[test]
fn should_refine_once_a_copy_comes_back_after_every_copy_went_down() {
    // Given: the only copy goes down at the first request and comes back later
    capturing_logger();
    let (cfg, mut listeners) = free_copies(1);
    let listener = listeners.remove(0);
    let port = listener.local_addr().unwrap().port();
    let posts = Posts::default();
    let counted = Arc::clone(&posts);
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
            counted.lock().unwrap().push(port);
            drop(listener);
            reset(stream);
            break;
        }
        thread::sleep(COMES_BACK_AFTER);
        let back = TcpListener::bind(("127.0.0.1", port)).unwrap();
        spawn_copy(back, Arc::new(vec![CopyTurn::Answer]), counted);
    });
    let mut refiner = OpenAiTextRefiner::connect(cfg)
        .unwrap()
        .with_recovery_wait(SHORT_RECOVERY_WAIT);
    // When
    let cleaned = refiner.refine(&RawTranscript::new(TRANSCRIPT.to_owned()).unwrap());
    // Then
    assert_eq!(
        cleaned.unwrap().as_str(),
        "The Docker image builds fine on staging."
    );
    assert_eq!(posts.lock().unwrap().len(), 2);
    assert_eq!(logged_by_this_thread(WAITING_FOR_A_COPY), 1);
}
