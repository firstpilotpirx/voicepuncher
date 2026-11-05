//! The words said before the last report are decided; the ones after it are judged anew.

use super::*;

#[test]
fn should_skip_words_said_before_the_last_report() {
    // Given "stop stop" reported up to 1.55 s
    let words = said(1.0, "stop stop");
    // When
    let reported = Reported { until: 1.55, commands: vec!["stop dictation".into()] };
    let again = decide(&dictating(), &words, Moment::Closing, &reported);
    // Then
    assert_eq!(again, None);
}

#[test]
fn should_report_the_next_command_said_after_the_last_report() {
    // Given "press enter" reported up to 1.55 s, then "stop" ending at 1.85 s, the gate's
    // pause begun right after it
    let words = said(1.0, "press enter stop");
    // When
    let paused = Moment::Decoding { now: 2.1, pause: Some(1.9), synced: false };
    let reported = Reported { until: 1.55, commands: vec!["press enter".into()] };
    let next = decide(&dictating(), &words, paused, &reported);
    // Then
    assert_eq!(
        next.expect("the stop after it").commands,
        ["stop dictation"]
    );
}
