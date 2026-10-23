//! Confirmations on a loaded machine: every copy decodes slowly, and the budget follows.

use super::*;

/// A live copy on a port of its own, decoding as `decodings` say, and its transcriber.
fn transcriber_of_live_copy(decodings: Vec<Duration>) -> (UtteranceTranscriber, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let requests = serve_live_copy_paced(listener, PRESS_ENTER, decodings);
    (UtteranceTranscriber::new(&url).unwrap(), requests)
}

#[test]
fn should_hear_nothing_when_a_live_copy_decodes_past_the_budget() {
    // Given: the recent confirmations came back quickly
    let lines = crate::test_log::captured();
    let (transcriber, requests) = transcriber_of_live_copy(vec![
        Duration::ZERO,
        Duration::ZERO,
        Duration::from_millis(1_500),
        Duration::ZERO,
    ]);
    warmed_up(&transcriber, Duration::from_secs(3));
    // When
    let (heard, took) = timed(&transcriber, Duration::from_secs(3));
    let heard = heard.unwrap();
    // Then
    assert_eq!(heard, Heard::default());
    assert!(
        took >= Duration::from_millis(900) && took < Duration::from_millis(1_400),
        "took {took:?}"
    );
    let cut = lines.containing("still decoding, nothing heard");
    assert!(cut.iter().any(|line| line.contains("127.0.0.1")), "{cut:?}");
    assert_eq!(requests.load(Ordering::SeqCst), 3);
    // And the next confirmation goes to that copy at once
    let (next, took) = timed(&transcriber, Duration::from_secs(3));
    assert_eq!(next.unwrap().text, " press enter");
    assert!(took < Duration::from_millis(500), "took {took:?}");
    assert_eq!(requests.load(Ordering::SeqCst), 4);
}

/// Two confirmations of silence with `deadline`, as the recent ones the pace follows.
fn warmed_up(transcriber: &UtteranceTranscriber, deadline: Duration) {
    for _ in 0..2 {
        let _ = timed(transcriber, deadline);
    }
}

/// A confirmation of silence with `deadline`, and how long it took.
fn timed(transcriber: &UtteranceTranscriber, deadline: Duration) -> (Result<Heard>, Duration) {
    let started = Instant::now();
    let heard = transcriber.transcribe(&[0.0; 1_600], SAMPLE_RATE, deadline);
    (heard, started.elapsed())
}

/// How long a copy on a loaded machine takes for an ordinary confirmation: close to the
/// budget of an idle machine.
const LOADED_ANSWER: Duration = Duration::from_millis(900);

#[test]
fn should_hear_in_full_a_live_copy_that_decodes_slowly_after_slow_answers() {
    // Given a copy that took close to a second for its recent confirmations, as under load,
    // and keeps decoding the next one for 1.5 s
    let (transcriber, requests) = transcriber_of_live_copy(vec![
        LOADED_ANSWER,
        LOADED_ANSWER,
        Duration::from_millis(1_500),
    ]);
    warmed_up(&transcriber, Duration::from_secs(3));
    // When
    let (heard, _) = timed(&transcriber, Duration::from_secs(3));
    // Then it is answered, not cut short
    let heard = heard.unwrap();
    assert_eq!(heard.text, " press enter");
    assert_eq!(requests.load(Ordering::SeqCst), 3);
}

#[test]
fn should_cut_a_confirmation_short_at_its_deadline_however_slow_the_recent_answers() {
    // Given a copy that took longer than half the deadline for its recent confirmations and
    // keeps decoding the next one past the deadline
    let deadline = Duration::from_secs(2);
    let (transcriber, _) = transcriber_of_live_copy(vec![
        Duration::from_millis(1_200),
        Duration::from_millis(1_200),
        Duration::from_secs(5),
    ]);
    warmed_up(&transcriber, deadline);
    // When
    let (result, took) = timed(&transcriber, deadline);
    // Then it ends at the deadline as given up, not later and not as speech without a command
    assert!(result.is_err(), "{result:?}");
    assert!(
        took >= Duration::from_millis(1_900) && took < Duration::from_millis(2_600),
        "took {took:?}"
    );
}

#[test]
fn should_keep_the_budget_after_confirmations_cut_short() {
    // Given a copy that answered its recent confirmations quickly, then decodes each later
    // one for 2.5 s, as speech stuck repeating does, and the first of them was cut short
    let (transcriber, _) = transcriber_of_live_copy(vec![
        Duration::ZERO,
        Duration::ZERO,
        Duration::from_millis(2_500),
    ]);
    warmed_up(&transcriber, Duration::from_secs(3));
    let (first, _) = timed(&transcriber, Duration::from_secs(3));
    assert_eq!(first.unwrap(), Heard::default());
    for _ in 0..2 {
        // When
        let (heard, took) = timed(&transcriber, Duration::from_secs(3));
        // Then each is cut short after about 1.5 s, the budget never raised past it
        assert_eq!(heard.unwrap(), Heard::default());
        assert!(
            took >= Duration::from_millis(1_400) && took < Duration::from_millis(1_900),
            "took {took:?}"
        );
    }
}

/// What speech recognition answers when it gets stuck repeating: one word over and over, far
/// longer than any speech of a confirmation's length.
fn stuck_repeating() -> &'static str {
    let text = " E-e".repeat(60);
    Box::leak(format!(r#"{{"text":"{text}","segments":[]}}"#).into_boxed_str())
}

#[test]
fn should_keep_the_budget_after_an_answer_stuck_repeating() {
    // Given a copy that took close to a second for its recent confirmations, as under load,
    // then answered one after 1.6 s stuck repeating, and decodes the next one for 2.5 s
    let loaded = Duration::from_millis(950);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    serve_live_copy_scripted(
        listener,
        vec![
            (loaded, PRESS_ENTER),
            (loaded, PRESS_ENTER),
            (Duration::from_millis(1_600), stuck_repeating()),
            (Duration::from_millis(2_500), PRESS_ENTER),
        ],
    );
    let transcriber = UtteranceTranscriber::new(&url).unwrap();
    warmed_up(&transcriber, Duration::from_secs(3));
    let (repeating, _) = timed(&transcriber, Duration::from_secs(3));
    assert!(repeating.unwrap().text.len() > 120);
    // When
    let (heard, took) = timed(&transcriber, Duration::from_secs(3));
    // Then it is cut short after the 1.9 s the ordinary answers give, not heard at 2.5 s
    assert_eq!(heard.unwrap(), Heard::default());
    assert!(took < Duration::from_millis(2_300), "took {took:?}");
}
