//! The hotkey signals: what starts, stops and cancels a recording, and the run loop's end.

use std::sync::mpsc;

use super::*;
use crate::core::domain::{FailureNotice, Permission};

#[test]
fn should_stop_the_recording_on_released_in_hold_mode() {
    // Given
    let (paster, texts) = noting_paster();
    let mut pipeline = PipelineUnderTest::new()
        .paster(paster)
        .mode(RecordingMode::Hold)
        .build();
    press(&mut pipeline);
    // When
    signal(&mut pipeline, PttSignal::Released);
    // Then
    assert_eq!(seen(&texts), ["hello"]);
}

#[test]
fn should_drop_the_recording_when_too_short() {
    // Given
    let (overlay, shown) = noting_overlay();
    let (paster, texts) = noting_paster();
    let mut pipeline = PipelineUnderTest::new()
        .recorder(recording(too_short()))
        .overlay(overlay)
        .paster(paster)
        .build();
    press(&mut pipeline);
    // When — the second press stops in Toggle mode
    press(&mut pipeline);
    // Then — speech recognition shows at once on the stop, hidden after the length check
    assert_eq!(seen(&shown), ["recording", "recognizing speech", "hidden"]);
    assert!(seen(&texts).is_empty());
}

#[test]
fn should_ignore_a_release_and_a_cancel_while_idle() {
    // Given
    let mut sounds = MockSoundPlayer::new();
    sounds.expect_play_cancel().never();
    let mut pipeline = PipelineUnderTest::new()
        .recorder(idle_recorder())
        .sounds(sounds)
        .build();
    // When / Then — the recorder is never started
    signal(&mut pipeline, PttSignal::Released);
    signal(&mut pipeline, PttSignal::Cancel);
}

#[test]
fn should_show_recording_when_the_recording_starts() {
    // Given
    let (overlay, shown) = noting_overlay();
    let mut pipeline = PipelineUnderTest::new()
        .recorder(started_once())
        .overlay(overlay)
        .build();
    // When
    press(&mut pipeline);
    // Then
    assert_eq!(seen(&shown), ["recording"]);
}

#[test]
fn should_hide_the_overlay_and_paste_nothing_when_cancelled() {
    // Given
    let (overlay, shown) = noting_overlay();
    let (paster, texts) = noting_paster();
    let mut pipeline = PipelineUnderTest::new()
        .overlay(overlay)
        .paster(paster)
        .build();
    press(&mut pipeline);
    // When
    signal(&mut pipeline, PttSignal::Cancel);
    // Then — no processing sign
    assert_eq!(seen(&shown), ["recording", "hidden"]);
    assert!(seen(&texts).is_empty());
}

#[test]
fn should_play_the_cancel_cue_and_not_the_stop_cue_when_cancelled() {
    // Given
    let mut sounds = MockSoundPlayer::new();
    sounds.expect_play_start().once().return_const(());
    sounds.expect_play_stop().never();
    sounds.expect_play_cancel().once().return_const(());
    let mut pipeline = PipelineUnderTest::new().sounds(sounds).build();
    press(&mut pipeline);
    // When / Then — the cues are checked by the double
    signal(&mut pipeline, PttSignal::Cancel);
}

#[test]
fn should_return_from_run_on_shutdown_while_idle() {
    // Given
    let mut pipeline = PipelineUnderTest::new().recorder(idle_recorder()).build();
    let (inputs, rx) = mpsc::channel::<PipelineInput>();
    inputs.send(PipelineInput::Shutdown).unwrap();
    // When
    let result = pipeline.run(rx);
    // Then — the channel is still open, so only the shutdown ended the loop
    assert!(result.is_ok());
    drop(inputs);
}

#[test]
fn should_run_until_the_input_channel_closes() {
    // Given
    let (tx, rx) = mpsc::channel::<PipelineInput>();
    drop(tx);
    let mut pipeline = PipelineUnderTest::new().recorder(idle_recorder()).build();
    // When
    let result = pipeline.run(rx);
    // Then
    assert!(result.unwrap_err().to_string().contains("channel closed"));
}

/// Runs a pipeline that records once and then gets `input`; what the overlay showed.
fn run_recording_then(input: PipelineInput) -> Vec<String> {
    let (overlay, shown) = noting_overlay();
    let (paster, texts) = noting_paster();
    let mut sounds = MockSoundPlayer::new();
    sounds.expect_play_start().once().return_const(());
    sounds.expect_play_cancel().once().return_const(());
    let mut pipeline = PipelineUnderTest::new()
        .overlay(overlay)
        .paster(paster)
        .sounds(sounds)
        .cfg(PipelineConfig::default())
        .build();
    let (inputs, rx) = mpsc::channel::<PipelineInput>();
    inputs
        .send(PipelineInput::Hotkey(PttSignal::Pressed))
        .unwrap();
    inputs.send(input).unwrap();
    pipeline.run(rx).unwrap();
    assert!(seen(&texts).is_empty());
    seen(&shown)
}

#[test]
fn should_cancel_the_recording_on_shutdown() {
    assert_eq!(
        run_recording_then(PipelineInput::Shutdown),
        ["recording", "hidden"]
    );
}

#[test]
fn should_cancel_the_recording_with_a_notice_naming_a_revoked_permission() {
    let notice = FailureNotice::PermissionRevoked(Permission::Microphone);
    assert_eq!(
        run_recording_then(PipelineInput::PermissionRevoked(Permission::Microphone)),
        [
            "recording".to_owned(),
            "hidden".to_owned(),
            format!("{notice:?}")
        ]
    );
}
