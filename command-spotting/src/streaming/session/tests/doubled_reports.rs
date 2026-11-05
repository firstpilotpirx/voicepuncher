//! Both recognizers hear one command, the English one stamping it about a second after the
//! Russian one: one command said is reported once, and one said twice is reported twice.

use super::*;

/// How much later than the Russian recognizer the English one stamps a word: 0.65–1.1 s in the
/// service log of 2026-11-03/04.
const ENGLISH_LAG_SECS: f32 = 1.0;

#[test]
fn should_report_a_command_the_recognizers_time_a_second_apart_once() {
    // Given "энтер" at 0.6 s of the utterance's audio, stamped a second later by the English
    // recognizer
    let ru = Scripted::new("ru", &[("энтер", 0.6)]);
    let en = Scripted::new("en", &[("enter", 0.6 + ENGLISH_LAG_SECS)]);
    // When
    let events = utterance_events(&[&en, &ru], 1.0);
    // Then
    assert_eq!(commands(&events), [["press enter"]], "{events:?}");
}
