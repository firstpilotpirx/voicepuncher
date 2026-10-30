//! The pause flushed into the recognizers: silence fed ahead of the stream at the gate's pause,
//! so a recognizer that emits a word's last sound late emits it at once.

use super::super::utterance::FLUSH_SECS;
use super::*;

/// The start of the first command in `events`.
fn command_start(events: &[ServerEvent]) -> Option<f32> {
    events.iter().find_map(|e| match e {
        ServerEvent::Command { start, .. } => Some(*start),
        _ => None,
    })
}

#[test]
fn should_report_a_lone_stop_at_the_pause_from_a_recognizer_that_emits_it_late() {
    // Given "стоп" said at 1.1 s to a recognizer that emits a word 0.6 s of audio after it
    let ru = Scripted::new("ru", &[("стоп", 0.6)]).lagging(0.6);
    let mut session = session(&[&ru], None);
    // When the stream reaches 0.2 s into the pause after it
    let mut events = session.accept(&[silence(1.0), speech(0.3), silence(0.2)].concat());
    events.extend(session.sync(0));
    // Then
    assert_eq!(commands(&events), [["stop dictation"]], "{events:?}");
}

/// How long after the speech's onset "press enter" starts: said 0.7 s after the onset at 1.0 s,
/// after a pause from 1.3 s to 1.6 s, streamed in `chunk` samples at a time; the recognizer's
/// own times, from the start of the replay, count one flush.
fn press_enter_after_a_pause(chunk: usize) -> f32 {
    let at = |after_onset: f32| 0.5 + after_onset + FLUSH_SECS;
    let en = Scripted::new("en", &[("press", at(0.7)), ("enter", at(1.0))]);
    let mut session = session(&[&en], None);
    let audio = [silence(1.0), speech(0.3), silence(0.3), speech(0.6)].concat();
    let mut events: Vec<ServerEvent> = audio
        .chunks(chunk)
        .flat_map(|c| session.accept(c))
        .collect();
    events.extend(session.sync(0));
    let onset = events.iter().find_map(|e| match e {
        ServerEvent::SpeechStart { t } => Some(*t),
        _ => None,
    });
    command_start(&events).expect("a command") - onset.expect("an onset")
}

#[test]
fn should_place_a_command_after_a_pause_at_its_time_in_the_stream() {
    // When
    let start = press_enter_after_a_pause(1_600);
    // Then
    assert!((start - 0.7).abs() < 1e-3, "{start}");
}

#[test]
fn should_flush_no_silence_into_speech_streamed_in_frames_shorter_than_the_gates_window() {
    // When the audio comes 20 ms at a time, shorter than the gate's window
    let start = press_enter_after_a_pause(320);
    // Then
    assert!((start - 0.7).abs() < 1e-3, "{start}");
}
