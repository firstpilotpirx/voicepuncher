// Integration tests for OpenAiTranscriber against an in-test HTTP stub.
// Two tests are gated on VOICEPUNCHER_TRANSCRIPTION_URL (+ VOICEPUNCHER_BENCH_CORPUS)
// and transcribe a corpus WAV, clean and under high noise, through a real speech
// recognition service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use voicepuncher::adapters::secondary::recording::filesystem::FileSystemAudioRecorder;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::Transcriber;
use voicepuncher::core::domain::AudioBuffer16k;
use voicepuncher_copy_pool::ServiceAddress;

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

/// The lines any thread logged that contain `fragment`: the pool logs from its own threads.
fn logged_by_any_thread(fragment: &str) -> Vec<String> {
    let lines = capturing_logger().0.lock().unwrap();
    lines
        .iter()
        .filter(|(_, line)| line.contains(fragment))
        .map(|(_, line)| line.clone())
        .collect()
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
    let (cfg, mut listeners) = free_copies(2);
    let other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    let hung = spawn_copy(listeners.remove(0), vec![CopyTurn::Hang]);
    let cfg = TranscriptionCfg {
        timeout_secs: SHORT_TIMEOUT_SECS,
        ..cfg
    };
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(hung.requests() + other.requests(), 2);
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

/// The request timeout of the give-up test: shorter than the wait for a short recording, so
/// each of the three attempts is given up after it and the test waits seconds, not half a minute.
const GIVE_UP_TIMEOUT_SECS: u64 = 1;
/// Three attempts of `GIVE_UP_TIMEOUT_SECS` each and their pauses: the cycle is dropped at once
/// after the last attempt.
const GIVEN_UP_WITHIN: Duration = Duration::from_secs(5);

#[test]
fn should_give_up_a_hung_short_recording_after_three_attempts_within_their_waits() {
    // Given
    capturing_logger();
    let (mut cfg, attempts) = spawn_scripted_stub(vec![Turn::Hang]);
    cfg.timeout_secs = GIVE_UP_TIMEOUT_SECS;
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    // When
    let started = Instant::now();
    let result = transcriber.transcribe(&five_seconds());
    // Then
    assert!(result.is_err());
    assert!(
        started.elapsed() < GIVEN_UP_WITHIN,
        "held for {:?}",
        started.elapsed()
    );
    // Each request ends at its own short timeout and frees the only copy, so every attempt
    // reaches it
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(logged_by_this_thread("WARN speech recognition service"), 3);
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

/// The real service's URL and the corpus's first recording with its manifest entry, or `None`
/// (with a GATED message) when the service URL, the corpus or its manifest is missing.
fn real_service_and_first_corpus_sample() -> Option<(String, serde_json::Value, AudioBuffer16k)> {
    let Ok(base_url) = std::env::var("VOICEPUNCHER_TRANSCRIPTION_URL") else {
        eprintln!("GATED: VOICEPUNCHER_TRANSCRIPTION_URL not set, skipping");
        return None;
    };
    let Ok(corpus) = std::env::var("VOICEPUNCHER_BENCH_CORPUS") else {
        eprintln!("GATED: VOICEPUNCHER_BENCH_CORPUS not set, skipping");
        return None;
    };
    let Ok(manifest) = std::fs::read_to_string(PathBuf::from(&corpus).join("manifest.jsonl"))
    else {
        eprintln!("GATED: no manifest.jsonl in VOICEPUNCHER_BENCH_CORPUS, skipping");
        return None;
    };
    let first: serde_json::Value = serde_json::from_str(manifest.lines().next().unwrap()).unwrap();
    let wav = PathBuf::from(&corpus).join(first["path"].as_str().unwrap());
    let audio = FileSystemAudioRecorder::load(&wav).unwrap();
    Some((base_url, first, audio))
}

fn transcribe_through_real_service(base_url: &str, audio: &AudioBuffer16k) -> String {
    let cfg = TranscriptionCfg {
        base_url: base_url.to_owned(),
        ..TranscriptionCfg::default()
    };
    let transcript = OpenAiTranscriber::connect(cfg)
        .unwrap()
        .transcribe(audio)
        .unwrap();
    transcript
        .expect("speech should not be discarded")
        .as_str()
        .to_owned()
}

/// The peak of the white noise laid over speech: loud enough that the service is unsure of
/// windows, which made it fall back to random sampling, yet quiet enough that the speech is
/// still kept (a peak of 6000 buried it: the transcript was discarded as no speech).
const HIGH_NOISE_PEAK: i32 = 1_500;
const NOISE_SEED: u32 = 0x5eed_1234;

/// Lays reproducible white noise (a fixed-seed linear congruential generator) over the speech.
fn under_high_noise(speech: &AudioBuffer16k) -> AudioBuffer16k {
    const LCG_MULTIPLIER: u32 = 1_664_525;
    const LCG_INCREMENT: u32 = 1_013_904_223;
    let mut state = NOISE_SEED;
    let noisy = speech
        .samples()
        .iter()
        .map(|&sample| {
            state = state
                .wrapping_mul(LCG_MULTIPLIER)
                .wrapping_add(LCG_INCREMENT);
            let noise = (state >> 16) as i32 % (2 * HIGH_NOISE_PEAK + 1) - HIGH_NOISE_PEAK;
            (i32::from(sample) + noise).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
        })
        .collect();
    AudioBuffer16k::new(noisy)
}

#[test]
fn should_transcribe_corpus_sample_through_real_service() {
    // Given — gated on a running service and the corpus
    let Some((base_url, first, audio)) = real_service_and_first_corpus_sample() else {
        return;
    };
    // When
    let text = transcribe_through_real_service(&base_url, &audio).to_lowercase();
    // Then
    let reference = first["reference"].as_str().unwrap();
    let last_word = reference.split_whitespace().last().unwrap();
    assert!(
        text.contains(last_word),
        "expected '{last_word}' in '{text}'"
    );
}

/// Answers the requests in turn with `answers` (the last one repeats) and passes each
/// request on, so a test sees what every attempt asked for.
fn spawn_answering_stub(
    answers: Vec<&'static str>,
) -> (TranscriptionCfg, mpsc::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut answered = 0usize;
        for stream in listener.incoming() {
            let answer = answers[answered.min(answers.len() - 1)];
            let Some(request) = read_request(&mut stream.unwrap(), &StubReply::ok(answer)) else {
                continue;
            };
            answered += 1;
            let _ = tx.send(request);
        }
    });
    let cfg = TranscriptionCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..TranscriptionCfg::default()
    };
    (cfg, rx)
}

const KOREAN_ANSWER: &str = r#"{"text": " 네, 좋아요.", "language": "korean"}"#;
const RUSSIAN_ANSWER: &str = r#"{"text": " Да, хорошо.", "language": "russian"}"#;
const ENGLISH_ANSWER: &str = r#"{"text": " Restart the server.", "language": "english"}"#;
/// Long enough for a second request the transcriber would have sent.
const NO_MORE_REQUESTS_WITHIN: Duration = Duration::from_millis(300);

fn next_request(requests: &mpsc::Receiver<CapturedRequest>) -> CapturedRequest {
    requests.recv_timeout(Duration::from_secs(5)).unwrap()
}

#[test]
fn should_request_a_transcript_in_another_language_again_with_the_first_user_language() {
    // Given
    capturing_logger();
    let (cfg, requests) = spawn_answering_stub(vec![KOREAN_ANSWER, RUSSIAN_ANSWER]);
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("Да, хорошо."));
    let first = next_request(&requests);
    assert!(!first.body.contains("name=\"language\""));
    assert!(first
        .body
        .contains("name=\"response_format\"\r\n\r\nverbose_json\r\n"));
    let retry = next_request(&requests);
    assert!(retry.body.contains("name=\"language\"\r\n\r\nru\r\n"));
    assert_eq!(logged_by_this_thread("korean"), 1);
}

#[test]
fn should_keep_a_transcript_detected_as_one_of_the_user_languages() {
    // Given
    let (cfg, requests) = spawn_answering_stub(vec![ENGLISH_ANSWER, RUSSIAN_ANSWER]);
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("Restart the server."));
    let first = next_request(&requests);
    assert!(first
        .body
        .contains("name=\"response_format\"\r\n\r\nverbose_json\r\n"));
    assert!(requests.recv_timeout(NO_MORE_REQUESTS_WITHIN).is_err());
}

#[test]
fn should_keep_a_transcript_of_a_fixed_language_whatever_the_service_detects() {
    // Given
    let (cfg, requests) = spawn_answering_stub(vec![KOREAN_ANSWER, RUSSIAN_ANSWER]);
    let cfg = TranscriptionCfg {
        language: "en".into(),
        ..cfg
    };
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("네, 좋아요."));
    next_request(&requests);
    assert!(requests.recv_timeout(NO_MORE_REQUESTS_WITHIN).is_err());
}

#[test]
fn should_transcribe_the_same_noisy_recording_to_the_same_text_twice() {
    // Given — gated on a running service and the corpus
    let Some((base_url, _, speech)) = real_service_and_first_corpus_sample() else {
        return;
    };
    let noisy = under_high_noise(&speech);
    // When
    let first = transcribe_through_real_service(&base_url, &noisy);
    let second = transcribe_through_real_service(&base_url, &noisy);
    // Then
    assert_eq!(first, second);
}

/// What one fake copy does with one request.
#[derive(Clone, Copy)]
enum CopyTurn {
    /// Answers "hello" after `COPY_WORK`.
    Answer,
    /// Reads the request and never answers it.
    Hang,
    /// Resets the connection as the request reaches it, as a killed copy does.
    Reset,
}

/// How long a fake copy works on a request it answers.
const COPY_WORK: Duration = Duration::from_millis(300);

/// A fake copy of the service: counts the requests it got and how many it held at once.
struct FakeCopy {
    port: u16,
    requests: Arc<AtomicUsize>,
    most_at_once: Arc<AtomicUsize>,
}

impl FakeCopy {
    fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    fn most_at_once(&self) -> usize {
        self.most_at_once.load(Ordering::SeqCst)
    }
}

/// Listeners on the first `n` copy ports of a service address nothing listens on.
fn free_copies(n: usize) -> (TranscriptionCfg, Vec<TcpListener>) {
    loop {
        let base = reserve_port();
        let base_url = format!("http://127.0.0.1:{base}/v1");
        let ports = ServiceAddress::parse(&base_url).unwrap().copy_ports();
        let listeners: Result<Vec<_>, _> = ports[..n]
            .iter()
            .map(|port| TcpListener::bind(("127.0.0.1", *port)))
            .collect();
        if let Ok(listeners) = listeners {
            let cfg = TranscriptionCfg {
                base_url,
                ..TranscriptionCfg::default()
            };
            return (cfg, listeners);
        }
    }
}

/// The service ports the fake copies' addresses take: their copies land on 21001–28000, clear of
/// the system's ephemeral ports (49152+), which the stubs of the other tests take and whose
/// pools go on probing ports they freed, and clear of the user's copies (18082–18089) and the
/// test services (28080–28085, 38082–38089).
const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 11_000..18_000;

/// A port of `FAKE_SERVICE_PORTS` bound without listening for the life of the test process:
/// connections to it are refused, so the pool looks for copies, and no test running beside this
/// one binds it as a copy of its own.
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

/// The index of the copy this process tries first among `n` copies.
fn first_tried(n: usize) -> usize {
    std::process::id() as usize % n
}

/// Serves each request on its own thread as `script` says; the last turn repeats.
/// Connections that send nothing (the pool's probes) are not counted.
fn spawn_copy(listener: TcpListener, script: Vec<CopyTurn>) -> FakeCopy {
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(AtomicUsize::new(0));
    let most_at_once = Arc::new(AtomicUsize::new(0));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let copy = FakeCopy {
        port,
        requests: Arc::clone(&requests),
        most_at_once: Arc::clone(&most_at_once),
    };
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let (requests, most_at_once, in_flight) = (
                Arc::clone(&requests),
                Arc::clone(&most_at_once),
                Arc::clone(&in_flight),
            );
            let script = script.clone();
            thread::spawn(move || {
                let mut first = [0u8; 1];
                if stream.peek(&mut first).unwrap_or(0) == 0 {
                    return;
                }
                let n = requests.fetch_add(1, Ordering::SeqCst);
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                most_at_once.fetch_max(now, Ordering::SeqCst);
                match script[n.min(script.len() - 1)] {
                    CopyTurn::Answer => {
                        let reply = StubReply {
                            delay: COPY_WORK,
                            ..StubReply::ok(r#"{"text": "hello"}"#)
                        };
                        read_request(&mut stream, &reply);
                        in_flight.fetch_sub(1, Ordering::SeqCst);
                    }
                    CopyTurn::Hang => {
                        let mut buf = [0u8; 65536];
                        let _ = stream.read(&mut buf);
                        thread::sleep(Duration::from_secs(600));
                    }
                    CopyTurn::Reset => {
                        in_flight.fetch_sub(1, Ordering::SeqCst);
                        reset(stream);
                    }
                }
            });
        }
    });
    copy
}

#[test]
fn should_send_requests_from_two_clients_at_once_to_two_copies_and_queue_the_rest() {
    // Given
    let (cfg, listeners) = free_copies(2);
    let copies: Vec<FakeCopy> = listeners
        .into_iter()
        .map(|listener| spawn_copy(listener, vec![CopyTurn::Answer]))
        .collect();
    // When
    let started = Instant::now();
    let clients: Vec<_> = (0..4)
        .map(|_| {
            let cfg = cfg.clone();
            thread::spawn(move || transcribe(cfg))
        })
        .collect();
    let transcripts: Vec<_> = clients.into_iter().map(|c| c.join().unwrap()).collect();
    // Then
    assert!(
        started.elapsed() >= COPY_WORK * 2,
        "four requests on two copies took {:?}: two of them did not wait their turn",
        started.elapsed()
    );
    for transcript in transcripts {
        assert_eq!(transcript.unwrap().as_deref(), Some("hello"));
    }
    for copy in &copies {
        assert!(copy.requests() >= 1, "copy {} got no request", copy.port);
        assert_eq!(
            copy.most_at_once(),
            1,
            "copy {} held two requests",
            copy.port
        );
    }
    assert_eq!(copies.iter().map(FakeCopy::requests).sum::<usize>(), 4);
}

#[test]
fn should_skip_a_copy_closed_before_a_request() {
    // Given
    let (cfg, mut listeners) = free_copies(2);
    let other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    capturing_logger();
    let closed = listeners.remove(0);
    let closed_port = closed.local_addr().unwrap().port();
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    drop(closed);
    // When
    let transcript = transcriber.transcribe(&half_second()).unwrap();
    // Then
    assert_eq!(transcript.unwrap().as_str(), "hello");
    assert_eq!(other.requests(), 1);
    let skipped = format!("copy {closed_port} refused the connection: skipped");
    assert_eq!(logged_by_any_thread(&skipped).len(), 1, "{skipped}");
}

#[test]
fn should_fail_naming_the_copies_when_every_copy_refuses() {
    // Given
    let (cfg, listeners) = free_copies(2);
    let ports: Vec<u16> = listeners
        .iter()
        .map(|l| l.local_addr().unwrap().port())
        .collect();
    capturing_logger();
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    drop(listeners);
    // When
    let message = format!("{:#}", transcriber.transcribe(&half_second()).unwrap_err());
    // Then
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service"),
        4,
        "not every attempt was used"
    );
    assert_eq!(logged_by_this_thread("— giving up"), 1);
    for port in ports {
        assert!(message.contains(&port.to_string()), "{message}");
    }
}

#[test]
fn should_send_the_attempt_after_a_hung_one_to_another_copy() {
    // Given
    let (cfg, mut listeners) = free_copies(2);
    let other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    let hung = spawn_copy(listeners.remove(0), vec![CopyTurn::Hang]);
    let cfg = TranscriptionCfg {
        timeout_secs: SHORT_TIMEOUT_SECS,
        ..cfg
    };
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(hung.requests(), 1, "the hung copy got a second attempt");
    assert_eq!(other.requests(), 1);
}

#[test]
fn should_log_a_copy_hung_with_the_time_waited_only_past_twice_its_time_limit() {
    // Given
    let (cfg, mut listeners) = free_copies(2);
    let _other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    // The default timeout: the request's own limit (a few seconds for half a second of audio)
    // is far below the timeout the copy's thread reads under, so the copy stays held
    let hung = spawn_copy(listeners.remove(0), vec![CopyTurn::Hang]);
    capturing_logger();
    let hung_line = format!("copy {} hung: no answer in ", hung.port);
    // When
    let started = Instant::now();
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    transcriber.transcribe(&half_second()).unwrap();
    // Then
    let limit = attempt_deadline_logged_by_this_thread();
    let logged_at = loop {
        if !logged_by_any_thread(&hung_line).is_empty() {
            break started.elapsed();
        }
        assert!(
            started.elapsed() < limit * 3 + Duration::from_secs(2),
            "copy {} was never logged as hung",
            hung.port
        );
        thread::sleep(Duration::from_millis(50));
    };
    assert!(
        logged_at >= limit * 2,
        "logged hung after {logged_at:?}, before twice its limit {limit:?}"
    );
    let line = logged_by_any_thread(&hung_line).remove(0);
    let waited: u64 = line
        .split(&hung_line)
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|secs| secs.parse().ok())
        .unwrap_or_else(|| panic!("no time waited in {line:?}"));
    assert!(
        Duration::from_secs(waited + 1) > limit * 2,
        "{line:?} names less than twice the limit {limit:?}"
    );
}

/// The time limit of the first attempt of this thread's request, from its warning.
fn attempt_deadline_logged_by_this_thread() -> Duration {
    let me = thread::current().id();
    let lines = capturing_logger().0.lock().unwrap();
    let ms: u64 = lines
        .iter()
        .filter(|(thread, line)| *thread == me && line.contains("attempt 1,"))
        .find_map(|(_, line)| {
            let after = line.split("deadline ").nth(1)?;
            after.split("ms").next()?.parse().ok()
        })
        .expect("the first attempt was logged with its deadline");
    Duration::from_millis(ms)
}

#[test]
fn should_transcribe_a_chunk_by_another_copy_when_its_copy_is_killed() {
    // Given
    let (cfg, mut listeners) = free_copies(2);
    let other = spawn_copy(listeners.remove(1 - first_tried(2)), vec![CopyTurn::Answer]);
    let killed = spawn_copy(listeners.remove(0), vec![CopyTurn::Reset]);
    capturing_logger();
    // When
    let transcript = transcribe(cfg).unwrap();
    // Then
    assert_eq!(transcript.as_deref(), Some("hello"));
    assert_eq!(killed.requests(), 1, "the killed copy got a second attempt");
    assert_eq!(
        logged_by_this_thread("WARN speech recognition service dropped the connection"),
        1
    );
    assert_eq!(other.requests(), 1);
}
