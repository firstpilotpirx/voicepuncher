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

#[test]
fn should_report_the_tail_of_a_chain_said_after_the_last_report() {
    // Given "stop enter" reported up to 3.06 s, and "start" decoded after that report
    let words = that_is_all_stop_enter_start();
    // When
    let found = decide(&dictating(), &words, paused_at(3.24, 3.22), 3.06);
    // Then the chain's last command is reported, from its own word
    let found = found.expect("the tail of the chain");
    assert_eq!(found.commands, ["start dictation"]);
    assert!((found.start - 3.20).abs() < 1e-4, "{}", found.start);
}

#[test]
fn should_report_nothing_of_a_chain_reported_whole() {
    // Given the whole chain reported up to the end of "start"
    let words = that_is_all_stop_enter_start();
    // When
    let found = decide(&dictating(), &words, Moment::Closing, 3.24);
    // Then
    assert_eq!(found, None);
}
