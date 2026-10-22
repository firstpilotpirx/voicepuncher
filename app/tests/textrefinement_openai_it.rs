// Integration tests for OpenAiTextRefiner against an in-test HTTP stub.
// One test is gated on VOICEPUNCHER_REFINEMENT_URL and refines a sentence through a
// real text refinement service.

use std::io::Write;
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

#[path = "support/refinement_stub.rs"]
mod refinement_stub;
use refinement_stub::{read_request, reserve_port, respond, CapturedRequest};

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
            (&raw const linger).cast(),
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

#[test]
fn should_connect_while_every_copy_is_busy_refining() {
    // Given: both copies are busy with a refinement that takes longer than the startup check
    let (cfg, posts) = copies_with(vec![CopyTurn::Silent(Duration::from_secs(4))]);
    let cfg = RefinementCfg { timeout_secs: 10, ..cfg };
    let busy: Vec<_> = (0..2)
        .map(|_| {
            let cfg = cfg.clone();
            thread::spawn(move || refine(cfg, TRANSCRIPT))
        })
        .collect();
    while posts.lock().unwrap().len() < 2 {
        thread::sleep(Duration::from_millis(20));
    }
    // When
    let started = std::time::Instant::now();
    let connected = OpenAiTextRefiner::connect(cfg);
    // Then
    assert!(connected.is_ok(), "{:?}", connected.err());
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "waited {:?}",
        started.elapsed()
    );
    for refinement in busy {
        refinement.join().unwrap().unwrap();
    }
}
