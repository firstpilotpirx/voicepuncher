// Integration tests for OpenAiTranscriber against an in-test HTTP stub.
// One test is gated on VOICEPUNCHER_TRANSCRIPTION_URL (+ VOICEPUNCHER_BENCH_CORPUS)
// and transcribes a corpus WAV through a real speech recognition service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, ThreadId};
use std::time::Duration;

use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::Transcriber;
use voicepuncher::core::domain::AudioBuffer16k;

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
    content_type: String,
    body: String,
}

/// Serves exactly one HTTP request; connections that close without sending
/// anything (the reachability probe) are skipped.
fn spawn_stub(reply: StubReply) -> (TranscriptionCfg, mpsc::Receiver<CapturedRequest>) {
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
    let cfg = TranscriptionCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..TranscriptionCfg::default()
    };
    (cfg, rx)
}

fn read_request(stream: &mut TcpStream, reply: &StubReply) -> Option<CapturedRequest> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let (mut content_length, mut content_type) = (0usize, String::new());
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
            "content-type" => content_type = value.trim().to_owned(),
            _ => {}
        }
    }
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

/// Resets the first `drops` requests (closes them without reading the body, so the OS
/// sends RST, as a busy service does), then answers with `reply`. Counts requests;
/// the reachability probe sends nothing and is not counted.
fn spawn_dropping_stub(drops: usize, reply: StubReply) -> (TranscriptionCfg, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut first = [0u8; 1];
            if stream.peek(&mut first).unwrap_or(0) == 0 {
                continue;
            }
            if counter.fetch_add(1, Ordering::SeqCst) < drops {
                drop(stream);
                continue;
            }
            read_request(&mut stream, &reply);
        }
    });
    let cfg = TranscriptionCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..TranscriptionCfg::default()
    };
    (cfg, attempts)
}

/// Answers every request with `body` on a connection it keeps open, as the real
/// service does. After the first answer it waits for a message on the returned
/// sender, resets that connection (as macOS reports a connection the service closed
/// minutes ago) and confirms on the returned receiver.
fn spawn_resetting_stub(
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
            while answer_kept_open(&mut stream, body) {
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
    let cfg = TranscriptionCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..TranscriptionCfg::default()
    };
    (cfg, reset_tx, done_rx)
}

/// Reads one request and answers it with `body`, leaving the connection open; false
/// when the client closed the connection instead.
fn answer_kept_open(stream: &mut TcpStream, body: &str) -> bool {
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

fn half_second() -> AudioBuffer16k {
    AudioBuffer16k::new(vec![100i16; 8_000])
}

fn transcribe(cfg: TranscriptionCfg) -> anyhow::Result<Option<String>> {
    let mut transcriber = OpenAiTranscriber::connect(cfg)?;
    let transcript = transcriber.transcribe(&half_second())?;
    Ok(transcript.map(|t| t.as_str().to_owned()))
}

#[test]
fn should_post_multipart_request_and_return_transcript() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(r#"{"text": " hello world\n"}"#));
    let cfg = TranscriptionCfg {
        language: "en".into(),
        ..cfg
    };
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello world"));
    let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        request.request_line,
        "POST /v1/audio/transcriptions HTTP/1.1"
    );
    assert!(request
        .content_type
        .starts_with("multipart/form-data; boundary="));
    for field in ["model", "language", "temperature", "response_format"] {
        assert!(
            request.body.contains(&format!("name=\"{field}\"")),
            "missing {field}"
        );
    }
    assert!(request.body.contains("name=\"language\"\r\n\r\nen\r\n"));
    assert!(request.body.contains("filename=\"audio.wav\""));
    assert!(request.body.contains("RIFF"));
}

#[test]
fn should_omit_language_when_auto() {
    // Given
    let (cfg, requests) = spawn_stub(StubReply::ok(r#"{"text": "привет"}"#));
    // When
    transcribe(cfg).unwrap();
    // Then
    let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!request.body.contains("name=\"language\""));
}

#[test]
fn should_join_word_split_across_segments() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(r#"{"text": " наход\nившаяся у власти"}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("находившаяся у власти"));
}

#[test]
fn should_discard_known_hallucination() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(
        r#"{"text": " Субтитры создавал DimaTorzok"}"#,
    ));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript, None);
}

#[test]
fn should_keep_speech_before_a_trailing_hallucination() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(
        r#"{"text": " I restarted the service.\n We split the module into several small parts.\n Thanks for watching.\n"}"#,
    ));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(
        transcript.as_deref(),
        Some("I restarted the service. We split the module into several small parts.")
    );
}

#[test]
fn should_remove_a_marker_inside_speech() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(
        r#"{"text": " the index\n [BLANK_AUDIO]\n was never built.\n"}"#,
    ));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("the index was never built."));
}

#[test]
fn should_return_no_transcript_when_service_hears_no_speech() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(r#"{"text": ""}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript, None);
}

#[test]
fn should_return_no_transcript_when_service_answers_only_whitespace() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok(r#"{"text": " \n"}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript, None);
}

#[test]
fn should_fail_on_server_error() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply {
        status: 500,
        ..StubReply::ok(r#"{"error": "boom"}"#)
    });
    // When / Then
    assert!(transcribe(cfg).is_err());
}

#[test]
fn should_fail_on_malformed_json() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply::ok("not json"));
    // When / Then
    let message = format!("{:#}", transcribe(cfg).unwrap_err());
    assert!(
        message.contains("unexpected speech recognition response"),
        "{message}"
    );
}

#[test]
fn should_fail_when_service_answers_after_timeout() {
    // Given
    let (cfg, _requests) = spawn_stub(StubReply {
        delay: Duration::from_secs(3),
        ..StubReply::ok(r#"{"text": "late"}"#)
    });
    let cfg = TranscriptionCfg {
        timeout_secs: 1,
        ..cfg
    };
    // When / Then
    assert!(transcribe(cfg).is_err());
}

#[test]
fn should_retry_when_service_drops_connection() {
    // Given
    capturing_logger();
    let (cfg, attempts) = spawn_dropping_stub(2, StubReply::ok(r#"{"text": "hello"}"#));
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service dropped the connection"),
        2
    );
}

#[test]
fn should_give_up_after_four_attempts_when_service_keeps_dropping() {
    // Given
    let (cfg, attempts) = spawn_dropping_stub(usize::MAX, StubReply::ok(r#"{"text": "x"}"#));
    // When
    let result = transcribe(cfg);
    // Then
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 4);
}

#[test]
fn should_send_a_recording_answered_with_errors_three_times() {
    // Given
    let (cfg, attempts) = spawn_dropping_stub(
        0,
        StubReply {
            status: 500,
            ..StubReply::ok(r#"{"error": "boom"}"#)
        },
    );
    // When
    let result = transcribe(cfg);
    // Then
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[test]
fn should_not_retry_a_request_the_service_cannot_take() {
    // Given
    let (cfg, attempts) = spawn_dropping_stub(
        0,
        StubReply {
            status: 400,
            ..StubReply::ok(r#"{"error": "bad request"}"#)
        },
    );
    // When
    let result = transcribe(cfg);
    // Then
    assert!(result.is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

/// What the scripted stub does with one request.
#[derive(Clone, Copy)]
enum Turn {
    /// Reads the request and never answers it.
    Hang,
    Answer(u16, &'static str),
}

/// Takes the requests in turn as `script` says; the last turn repeats. Counts requests.
fn spawn_scripted_stub(script: Vec<Turn>) -> (TranscriptionCfg, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&attempts);
    thread::spawn(move || {
        let mut held = Vec::new();
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut first = [0u8; 1];
            if stream.peek(&mut first).unwrap_or(0) == 0 {
                continue;
            }
            let n = counter.fetch_add(1, Ordering::SeqCst);
            match script[n.min(script.len() - 1)] {
                Turn::Hang => {
                    let mut buf = [0u8; 65536];
                    let _ = stream.read(&mut buf);
                    held.push(stream);
                }
                Turn::Answer(status, body) => {
                    read_request(
                        &mut stream,
                        &StubReply {
                            status,
                            ..StubReply::ok(body)
                        },
                    );
                }
            }
        }
    });
    let cfg = TranscriptionCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..TranscriptionCfg::default()
    };
    (cfg, attempts)
}

/// The longest the stubs below let one request take.
const SHORT_TIMEOUT_SECS: u64 = 2;

#[test]
fn should_transcribe_by_a_new_attempt_when_the_first_request_hangs() {
    // Given
    capturing_logger();
    let (cfg, attempts) =
        spawn_scripted_stub(vec![Turn::Hang, Turn::Answer(200, r#"{"text": "hello"}"#)]);
    let cfg = TranscriptionCfg {
        timeout_secs: SHORT_TIMEOUT_SECS,
        ..cfg
    };
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service did not answer in time"),
        1
    );
    assert_eq!(logged_by_this_thread("(request=1 attempt 1, after"), 1);
}

#[test]
fn should_transcribe_by_a_new_attempt_after_one_error_answer() {
    // Given
    capturing_logger();
    let (cfg, _attempts) = spawn_scripted_stub(vec![
        Turn::Answer(503, r#"{"error": "busy"}"#),
        Turn::Answer(200, r#"{"text": "hello"}"#),
    ]);
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service answered with an error"),
        1
    );
}

/// A recording of five seconds, as the spec's short recording.
fn five_seconds() -> AudioBuffer16k {
    AudioBuffer16k::new(vec![0; 5 * 16_000])
}

/// The longest the app may hold a short recording when the service never answers.
const GIVEN_UP_WITHIN: Duration = Duration::from_secs(30);

#[test]
fn should_give_up_a_short_recording_the_service_never_answers_within_half_a_minute() {
    // Given
    capturing_logger();
    let (cfg, attempts) = spawn_scripted_stub(vec![Turn::Hang]);
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    // When
    let started = std::time::Instant::now();
    let result = transcriber.transcribe(&five_seconds());
    // Then
    assert!(result.is_err());
    assert!(
        started.elapsed() < GIVEN_UP_WITHIN,
        "held for {:?}",
        started.elapsed()
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service did not answer in time"),
        3
    );
    assert_eq!(logged_by_this_thread("— giving up"), 1);
}

#[test]
fn should_transcribe_on_first_attempt_after_service_reset_idle_connection() {
    // Given
    capturing_logger();
    let (cfg, reset, reset_done) = spawn_resetting_stub(r#"{"text": "hello"}"#);
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    let first = transcriber.transcribe(&half_second()).unwrap();
    reset.send(()).unwrap();
    reset_done.recv_timeout(Duration::from_secs(5)).unwrap();
    thread::sleep(Duration::from_millis(50));
    // When
    let next = transcriber.transcribe(&half_second());
    // Then
    assert_eq!(first.unwrap().as_str(), "hello");
    assert_eq!(next.unwrap().unwrap().as_str(), "hello");
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service dropped the connection"),
        0
    );
}

#[test]
fn should_transcribe_corpus_sample_through_real_service() {
    // Given — gated on a running service and the corpus
    let Ok(base_url) = std::env::var("VOICEPUNCHER_TRANSCRIPTION_URL") else {
        eprintln!("GATED: VOICEPUNCHER_TRANSCRIPTION_URL not set, skipping");
        return;
    };
    let Ok(corpus) = std::env::var("VOICEPUNCHER_BENCH_CORPUS") else {
        eprintln!("GATED: VOICEPUNCHER_BENCH_CORPUS not set, skipping");
        return;
    };
    let manifest = std::fs::read_to_string(PathBuf::from(&corpus).join("manifest.jsonl")).unwrap();
    let first: serde_json::Value = serde_json::from_str(manifest.lines().next().unwrap()).unwrap();
    let wav = PathBuf::from(&corpus).join(first["path"].as_str().unwrap());
    let audio = FileSystemAudioRecorder::load(&wav).unwrap();
    let cfg = TranscriptionCfg {
        base_url,
        ..TranscriptionCfg::default()
    };
    // When
    let transcript = OpenAiTranscriber::connect(cfg)
        .unwrap()
        .transcribe(&audio)
        .unwrap();
    // Then
    let text = transcript
        .expect("speech should not be discarded")
        .as_str()
        .to_lowercase();
    let reference = first["reference"].as_str().unwrap();
    let last_word = reference.split_whitespace().last().unwrap();
    assert!(
        text.contains(last_word),
        "expected '{last_word}' in '{text}'"
    );
}
