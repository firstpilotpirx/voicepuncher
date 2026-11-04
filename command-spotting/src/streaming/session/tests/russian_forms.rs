//! Words the Russian recognizer writes for "enter" glued or mangled count as the command
//! when the English recognizer hears nothing.

use super::*;

/// Whether the first command in `events` came before the utterance's end of speech.
fn reported_before_speech_end(events: &[ServerEvent]) -> bool {
    let at = |is: fn(&ServerEvent) -> bool| events.iter().position(is);
    let command = at(|e| matches!(e, ServerEvent::Command { .. }));
    let end = at(|e| matches!(e, ServerEvent::SpeechEnd { .. }));
    matches!((command, end), (Some(c), Some(e)) if c < e)
}

#[test]
fn should_report_a_glued_enter_enter_the_russian_recognizer_writes_once() {
    // Given a sentence ending in one glued word for "enter enter", which only the Russian
    // recognizer hears
    let en = Scripted::new("en", &[]);
    let ru = Scripted::new("ru", &[("забей", 0.6), ("интернтер", 0.9)]);
    // When
    let events = utterance_events(&[&en, &ru], 0.8);
    // Then the doubled command is reported once, as soon as its word is heard
    assert_eq!(commands(&events), [["press enter"]]);
    assert!(reported_before_speech_end(&events), "{events:?}");
}

#[test]
fn should_report_a_mangled_enter_the_russian_recognizer_writes_alone() {
    // Given a mangled word for "enter" said alone, which only the Russian recognizer hears
    let en = Scripted::new("en", &[]);
    let ru = Scripted::new("ru", &[("нтр", 0.6)]);
    // When
    let events = utterance_events(&[&en, &ru], 0.5);
    // Then
    assert_eq!(commands(&events), [["press enter"]]);
}
