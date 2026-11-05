//! The tail of a chain decoded after its first commands were reported.

use super::*;

/// The English recognizer's words of the user's recorded "that is all stop enter start"
/// (session-4/5), stamped as the bench traced them: "start" came a decode after "enter".
fn that_is_all_stop_enter_start() -> Vec<TimedWord> {
    [
        ("That", 1.08, 1.18),
        ("is", 1.16, 1.26),
        ("all", 1.88, 1.98),
        ("stop", 2.24, 2.34),
        ("enter", 2.88, 3.06),
        ("start", 3.20, 3.24),
    ]
    .map(|(text, start, end)| TimedWord { text: text.into(), start, end })
    .to_vec()
}

/// The utterance's reports so far: ending at `until`, with `commands` sent.
fn reported(until: f32, commands: &[&str]) -> Reported {
    Reported { until, commands: commands.iter().map(|c| (*c).to_owned()).collect() }
}

#[test]
fn should_report_the_tail_of_a_chain_said_after_the_last_report() {
    // Given "stop enter" reported up to 3.06 s, and "start" decoded after that report
    let words = that_is_all_stop_enter_start();
    // When
    let reported = reported(3.06, &["stop dictation", "press enter"]);
    let found = decide(&dictating(), &words, paused_at(3.24, 3.22), &reported);
    // Then the chain's last command is reported, from its own word
    let found = found.expect("the tail of the chain");
    assert_eq!(found.commands, ["start dictation"]);
    assert!((found.start - 3.20).abs() < 1e-4, "{}", found.start);
}

#[test]
fn should_report_the_tail_of_a_chain_decoded_only_at_the_close() {
    // Given "stop enter" reported up to 3.06 s while the user spoke, and "start" decoded
    // only once the utterance ends
    let words = that_is_all_stop_enter_start();
    // When
    let reported = reported(3.06, &["stop dictation", "press enter"]);
    let found = decide(&dictating(), &words, Moment::Closing, &reported);
    // Then the chain's last command is reported alone
    let found = found.expect("the tail of the chain at the close");
    assert_eq!(found.commands, ["start dictation"]);
}

#[test]
fn should_report_nothing_of_a_chain_reported_whole() {
    // Given the whole chain reported up to the end of "start"
    let words = that_is_all_stop_enter_start();
    // When
    let reported = reported(3.24, &["stop dictation", "press enter", "start dictation"]);
    let found = decide(&dictating(), &words, Moment::Closing, &reported);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_report_nothing_of_the_words_of_a_chain_the_other_recognizer_reported() {
    // Given "stop enter" reported by the Russian recognizer, whose "энтер" ended at 2.5 s,
    // and the English recognizer, stamping later, decoding its "enter" from 2.88 s
    let words = &that_is_all_stop_enter_start()[..5];
    let reported = reported(2.5, &["stop dictation", "press enter"]);
    // When
    let found = decide(&dictating(), words, paused_at(3.3, 3.2), &reported);
    // Then its "enter" is the reported chain's, not a command said after it
    assert_eq!(found, None);
}

#[test]
fn should_report_nothing_of_the_english_enter_of_a_chain_reported_before_the_state_switched() {
    // Given "stop enter" reported by the Russian recognizer and the app switched to standby,
    // where "stop" is no command, and the English recognizer decoding its "enter" (E2E trace
    // of the user's session-4/5)
    let words: Vec<TimedWord> = [
        ("That", 1.72, 1.82),
        ("is", 1.88, 1.98),
        ("all", 2.60, 2.70),
        ("stop", 2.84, 2.94),
        ("enter", 3.16, 3.42),
    ]
    .map(|(text, start, end)| TimedWord { text: text.into(), start, end })
    .to_vec();
    let reported = reported(2.94, &["stop dictation", "press enter"]);
    // When, at the app's sync
    let moment = Moment::Decoding { now: 3.7, pause: None, synced: true };
    let found = decide(&standby(), &words, moment, &reported);
    // Then its "enter" is the reported chain's, told by the text, not a command said alone
    assert_eq!(found, None);
}
