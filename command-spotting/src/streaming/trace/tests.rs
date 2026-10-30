use std::time::Duration;

use super::*;
use crate::matcher::Placement;

pub fn press_enter(start: f32, end: f32) -> Finding {
    Finding { commands: vec!["press enter".into()], start, end, placement: Placement::Tail }
}

/// "press enter" heard by both, reported by `en` at 2.0 s, 0.3 s after its last word.
pub fn traced() -> Trace {
    let mut trace = Trace::new(1.0, 1.5, &["en", "ru"]);
    trace.heard(0, 1.8, "press");
    trace.heard(0, 1.9, "press enter");
    trace.heard(0, 2.0, "press enter");
    trace.heard(1, 2.0, "пресс энтер");
    trace.decoded(0, Duration::from_millis(30));
    trace.decoded(0, Duration::from_millis(12));
    trace.reported(0, &press_enter(1.5, 1.7), 2.0);
    trace.speech_end = 1.8;
    trace.closed = 2.6;
    trace
}

#[test]
fn should_keep_a_text_only_when_it_changed() {
    // When
    let trace = traced();
    // Then
    let texts: Vec<&str> = trace.recognizers[0]
        .texts
        .iter()
        .map(|t| t.text.as_str())
        .collect();
    assert_eq!(texts, ["press", "press enter"]);
}

#[test]
fn should_sum_decoding_and_keep_the_slowest_frame() {
    // When
    let trace = traced();
    // Then
    assert_eq!(trace.recognizers[0].decode_ms, 42);
    assert_eq!(trace.recognizers[0].slowest_frame_ms, 30);
}

#[test]
fn should_log_texts_decoding_and_each_report_with_its_delay() {
    // When
    let line = traced().log_line();
    // Then
    assert_eq!(
        line,
        "utterance 1.50–1.80s closed 2.60s | en \"press enter\" 42ms/30ms \
         | ru \"пресс энтер\" 0ms/0ms | press enter by en at 2.00s +300ms"
    );
}
