//! New attempts after a hung request or an error answer.

use super::*;

/// What the scripted stub does with one request.
#[derive(Clone, Copy)]
pub(super) enum Turn {
    /// Reads the request and never answers it.
    Hang,
    Answer(u16, &'static str),
}

/// Takes the requests in turn as `script` says; the last turn repeats. Counts requests.
pub(super) fn spawn_scripted_stub(script: Vec<Turn>) -> (TranscriptionCfg, Arc<AtomicUsize>) {
    spawn_counting_stub(move |n, mut stream| match script[n.min(script.len() - 1)] {
        Turn::Hang => {
            let mut buf = vec![0u8; 65536];
            let _ = stream.read(&mut buf);
            // Never answered and never closed: the client waits on it.
            std::mem::forget(stream);
        }
        Turn::Answer(status, body) => {
            read_request(&mut stream, &StubReply { status, ..StubReply::ok(body) });
        }
    })
}

/// The longest the stubs below let one request take.
pub(super) const SHORT_TIMEOUT_SECS: u64 = 2;

#[test]
pub(super) fn should_transcribe_by_a_new_attempt_when_the_first_request_hangs() {
    // Given
    capturing_logger();
    let (cfg, hung, other) = one_hung_copy_of_two();
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
pub(super) fn should_transcribe_by_a_new_attempt_after_one_error_answer() {
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
pub(super) fn five_seconds() -> AudioBuffer16k {
    AudioBuffer16k::new(vec![0; 5 * 16_000])
}

/// The request timeout of the give-up test: shorter than the wait for a short recording, so
/// each of the three attempts is given up after it and the test waits seconds, not half a minute.
pub(super) const GIVE_UP_TIMEOUT_SECS: u64 = 1;

/// Three attempts of `GIVE_UP_TIMEOUT_SECS` each and their pauses: the cycle is dropped at once
/// after the last attempt.
pub(super) const GIVEN_UP_WITHIN: Duration = Duration::from_secs(5);

#[test]
pub(super) fn should_give_up_a_hung_short_recording_after_three_attempts_within_their_waits() {
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
pub(super) fn should_transcribe_on_first_attempt_after_service_reset_idle_connection() {
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
