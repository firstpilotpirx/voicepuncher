//! The activity the pipeline publishes after every input.

use super::*;

#[test]
fn should_publish_recording_when_a_recording_starts() {
    // Given
    let (sink, published) = noting_activity();
    let mut pipeline = PipelineUnderTest::new()
        .recorder(started_once())
        .activity_sink(sink)
        .build();
    // When
    press(&mut pipeline);
    // Then
    assert_eq!(seen(&published), [PipelineActivity::Recording]);
}

#[test]
fn should_publish_processing_through_the_paste_then_idle() {
    // Given
    let (sink, published) = noting_activity();
    let mut pipeline = PipelineUnderTest::new().activity_sink(sink).build();
    press(&mut pipeline);
    // When — the second press stops in Toggle mode
    press(&mut pipeline);
    // Then
    assert_eq!(
        seen(&published),
        [
            PipelineActivity::Recording,
            PipelineActivity::Processing,
            PipelineActivity::Idle
        ]
    );
}

#[test]
fn should_publish_idle_after_a_cancel() {
    // Given
    let (sink, published) = noting_activity();
    let mut pipeline = PipelineUnderTest::new().activity_sink(sink).build();
    press(&mut pipeline);
    // When
    signal(&mut pipeline, PttSignal::Cancel);
    // Then
    assert_eq!(seen(&published).last(), Some(&PipelineActivity::Idle));
}

#[test]
fn should_publish_idle_for_a_signal_ignored_while_idle() {
    // Given
    let (sink, published) = noting_activity();
    let mut pipeline = PipelineUnderTest::new()
        .recorder(idle_recorder())
        .activity_sink(sink)
        .build();
    // When
    signal(&mut pipeline, PttSignal::Released);
    signal(&mut pipeline, PttSignal::Cancel);
    // Then — nothing started, nothing recorded
    assert_eq!(
        seen(&published),
        [PipelineActivity::Idle, PipelineActivity::Idle]
    );
}
