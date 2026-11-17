//! At an utterance's close, a recognizer that heard a whole command word speaks before one that
//! heard only its end: a quiet "restart" loses its "re" in one recognizer and is "start" there.

use super::*;

fn restart_session<'a>(models: &[&'a Scripted]) -> Session<'a> {
    session_listening(
        models,
        &cfg(None),
        &["start dictation", "restart dictation"],
    )
}

#[test]
fn should_report_restart_when_one_recognizer_heard_only_start_at_the_close() {
    // Given both recognizers emit their word only once the utterance closes
    let en = Scripted::new("en", &[("start", 0.7)]).lagging(10.0);
    let ru = Scripted::new("ru", &[("рестарт", 0.6)]).lagging(10.0);
    let mut session = restart_session(&[&en, &ru]);
    // When
    let mut events = session.accept(&[silence(1.0), speech(0.4), silence(1.0)].concat());
    events.extend(session.sync(0));
    // Then
    assert_eq!(commands(&events), [["restart dictation"]], "{events:?}");
}

#[test]
fn should_close_without_waiting_for_the_tail_of_silence_to_be_decoded() {
    // Given an English recognizer that takes 0.1 s per decode, and "stop" said at 1.1 s
    let en = Scripted::new("en", &[("stop", 0.6)]).slow(Duration::from_millis(100));
    let mut session = session(&[&en], None);
    let began = Instant::now();
    // When the audio streams in, the pause after it long enough to close the utterance
    let mut events = session.accept(&[silence(1.0), speech(0.3), silence(0.6)].concat());
    let accepted = began.elapsed();
    while commands(&events).is_empty() && began.elapsed() < Duration::from_secs(10) {
        events.extend(session.poll());
    }
    // Then the close waited for no decode, and the command came from a poll after it
    assert_eq!(commands(&events), [["stop dictation"]]);
    assert!(accepted < Duration::from_millis(500), "{accepted:?}");
}
