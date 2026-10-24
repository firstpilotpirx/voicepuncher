//! What the overlay shows along a dictation, and the live transcript while it records.

use std::sync::mpsc;
use std::time::Instant;

use super::*;
use crate::core::application::audio_recorder::LiveChunk;
use crate::core::application::live_transcript_view::MockLiveTranscriptView;
use crate::core::application::pipeline::live_transcript::LiveTranscript;
use crate::core::application::pipeline::live_worker::run_live_worker;
use crate::core::application::pipeline::SharedTranscriber;
use crate::core::domain::{FailureNotice, RawTranscript};

#[test]
fn should_show_speech_recognition_then_the_pasted_sign() {
    // Given
    let (overlay, shown) = noting_overlay();
    let mut pipeline = PipelineUnderTest::new().overlay(overlay).build();
    // When
    press(&mut pipeline);
    press(&mut pipeline);
    // Then
    assert_eq!(seen(&shown), ["recording", "recognizing speech", "pasted"]);
}

#[test]
fn should_end_on_the_pending_failure_notice_instead_of_the_pasted_sign() {
    // Given
    let (overlay, shown) = noting_overlay();
    let mut pipeline = PipelineUnderTest::new()
        .paster(paste_down())
        .overlay(overlay)
        .build();
    // When
    press(&mut pipeline);
    press(&mut pipeline);
    // Then
    let notice = FailureNotice::NotPasted(None);
    assert_eq!(seen(&shown).last(), Some(&format!("{notice:?}")));
    assert!(!seen(&shown).contains(&"pasted".to_owned()));
}

/// Runs the live worker over two chunks the transcriber answers with `answers` in turn.
fn run_worker_over(answers: Vec<Option<&'static str>>, view: MockLiveTranscriptView) {
    let mut transcriber = MockTranscriber::new();
    let mut answers = answers.into_iter();
    transcriber.expect_transcribe().returning(move |_| {
        Ok(answers
            .next()
            .flatten()
            .map(|t| RawTranscript::new(t.to_owned()).unwrap()))
    });
    let (chunk_tx, chunk_rx) = mpsc::channel();
    for _ in 0..2 {
        chunk_tx
            .send(LiveChunk { audio: one_second(), enqueued_at: Instant::now() })
            .unwrap();
    }
    drop(chunk_tx);
    let (results_tx, _results_rx) = mpsc::sync_channel(1);
    let (live_transcript, _recording_end) = LiveTranscript::new(Box::new(view));
    let transcriber: SharedTranscriber = Arc::new(Mutex::new(Box::new(transcriber)));
    run_live_worker(transcriber, chunk_rx, results_tx, live_transcript);
}

#[test]
fn should_show_the_stitched_text_after_every_chunk_with_text() {
    // Given
    let mut view = MockLiveTranscriptView::new();
    let mut seq = mockall::Sequence::new();
    view.expect_show_live_transcript()
        .withf(|text| text == "one two three")
        .once()
        .in_sequence(&mut seq)
        .return_const(());
    view.expect_show_live_transcript()
        .withf(|text| text == "one two three four five")
        .once()
        .in_sequence(&mut seq)
        .return_const(());
    // When / Then
    run_worker_over(
        vec![Some("one two three"), Some("two three four five")],
        view,
    );
}

#[test]
fn should_show_nothing_new_for_a_chunk_without_text() {
    // Given
    let mut view = MockLiveTranscriptView::new();
    view.expect_show_live_transcript()
        .withf(|text| text == "one two three")
        .once()
        .return_const(());
    // When / Then
    run_worker_over(vec![Some("one two three"), None], view);
}
