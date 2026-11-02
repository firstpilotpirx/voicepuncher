//! A command opening an utterance starts at the gate's onset: a recognizer times a first word
//! later than it is said, so the dictation it ends would keep the word's first sounds.

use super::*;

#[test]
fn should_start_a_command_opening_the_utterance_at_the_onset() {
    // Given "stop stop" from the onset at 1.0 s, timed 0.3 s late by the recognizer
    let en = Scripted::new("en", &[("stop", 0.8), ("stop", 1.0)]);
    let mut session = session(&[&en], None);
    // When
    let mut events = session.accept(&[silence(1.0), speech(0.6), silence(1.0)].concat());
    events.extend(session.sync(0));
    // Then
    let start = command_start(&events).expect("a command");
    assert!((start - 1.0).abs() < 1e-3, "{start}");
}

#[test]
fn should_start_a_command_after_other_words_at_its_own_time() {
    // Given "now enter enter", the command 0.4 s after the onset at 1.0 s
    let en = Scripted::new("en", &[("now", 0.6), ("enter", 0.9), ("enter", 1.1)]);
    let mut session = session(&[&en], None);
    // When
    let mut events = session.accept(&[silence(1.0), speech(0.8), silence(1.0)].concat());
    events.extend(session.sync(0));
    // Then
    let start = command_start(&events).expect("a command");
    assert!((start - 1.4).abs() < 1e-3, "{start}");
}

#[test]
fn should_replay_none_of_the_last_utterances_speech_into_the_next() {
    // Given an utterance closed after 0.4 s of quiet, and speech right after it
    let en = Scripted::new("en", &[]);
    let mut session = session(&[&en], None);
    // When
    let audio = [
        silence(1.0),
        speech(0.6),
        silence(0.4),
        speech(0.3),
        silence(1.0),
    ]
    .concat();
    session.accept(&audio);
    session.sync(0);
    // Then the next utterance hears only the quiet before it
    assert_eq!(en.streams.load(Ordering::SeqCst), 2);
    assert!(en.opening()[1] <= RATE * 2 / 5);
}
