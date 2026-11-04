//! An utterance closing while the next opens over its tail: the gate hears the first sound of
//! the next command late, so it lands in the closing utterance's last frames, where a
//! recognizer reads it as a lone command word; the utterance opened over it hears it whole.

use super::*;

#[test]
fn should_leave_a_word_after_its_speech_end_to_the_utterance_opened_over_it() {
    // Given "stop" heard at 1.95 s, in the closing quiet of an utterance whose speech ended at
    // 1.6 s, and the next utterance opening at 2.0 s over that quiet
    let en = Scripted::new("en", &[("stop", 2.05)]);
    let mut session = session(&[&en], None);
    let murmur = vec![0.05; RATE / 10];
    let audio = [
        silence(1.0),
        speech(0.6),
        silence(0.3),
        murmur,
        speech(0.3),
        silence(1.0),
    ]
    .concat();
    // When
    let mut events = session.accept(&audio);
    events.extend(session.sync(0));
    // Then the word is reported once, by the utterance opened over it
    assert_eq!(commands(&events), [["stop dictation"]], "{events:?}");
    let start = command_start(&events).expect("a command");
    assert!(start >= 2.0, "{start}");
}
