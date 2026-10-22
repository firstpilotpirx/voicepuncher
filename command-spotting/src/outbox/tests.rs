use super::*;

fn start(t: f32) -> ServerEvent {
    ServerEvent::SpeechStart { t }
}

fn end(t: f32) -> ServerEvent {
    ServerEvent::SpeechEnd { t }
}

fn phrase(name: &str) -> ServerEvent {
    ServerEvent::Phrase { phrase: name.to_owned(), start: 0.0, end: 0.0, timing: None }
}

fn timed_phrase(name: &str, timing: Timing) -> ServerEvent {
    ServerEvent::Phrase { phrase: name.to_owned(), start: 0.0, end: 0.0, timing: Some(timing) }
}

#[test]
fn should_send_a_held_segment_with_its_confirmation_after_it() {
    // Given
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(1, vec![start(1.0), end(2.0)]);
    // When
    let out = outbox.resolve(1, false, vec![phrase("press enter")]);
    // Then
    assert_eq!(out, [start(1.0), end(2.0), phrase("press enter")]);
}

#[test]
fn should_keep_a_later_segment_behind_an_earlier_one_still_waiting() {
    // Given
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(1, vec![start(1.0), end(2.0)]);
    outbox.asked(2, false);
    outbox.hold(2, vec![start(3.0), end(4.0)]);
    // When
    let early = outbox.resolve(2, false, Vec::new());
    let late = outbox.resolve(1, false, Vec::new());
    // Then
    assert!(early.is_empty());
    assert_eq!(late, [start(1.0), end(2.0), start(3.0), end(4.0)]);
}

#[test]
fn should_send_the_answer_of_a_check_at_once() {
    // Given
    let mut outbox = Outbox::default();
    outbox.asked(1, true);
    // When
    let out = outbox.resolve(1, true, vec![phrase("stop dictation")]);
    // Then
    assert_eq!(out, [phrase("stop dictation")]);
    assert!(!outbox.check_busy());
}

#[test]
fn should_send_other_events_after_the_segments_still_held() {
    // Given
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(1, vec![end(2.0)]);
    // When
    let early = outbox.send(vec![phrase("press enter")]);
    let out = outbox.resolve(1, false, Vec::new());
    // Then
    assert!(early.is_empty());
    assert_eq!(out, [end(2.0), phrase("press enter")]);
}

#[test]
fn should_put_a_phrase_spotted_after_its_held_segment_ended_before_that_end() {
    // Given
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(1, vec![start(1.0), end(2.0)]);
    // When
    let early = outbox.send_in_last_segment(vec![phrase("press enter")]);
    let out = outbox.resolve(1, false, Vec::new());
    // Then
    assert!(early.is_empty());
    assert_eq!(out, [start(1.0), phrase("press enter"), end(2.0)]);
}

#[test]
fn should_send_a_phrase_of_a_segment_already_out_at_once() {
    // When
    let out = Outbox::default().send_in_last_segment(vec![phrase("press enter")]);
    // Then
    assert_eq!(out, [phrase("press enter")]);
}

#[test]
fn should_drop_the_spotted_phrases_of_other_commands_from_a_held_segment() {
    // Given
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(
        1,
        vec![
            start(1.0),
            phrase("start dictation"),
            phrase("restart dictation"),
            end(2.0),
        ],
    );
    // When
    outbox.keep_only_phrase(1, "restart dictation", None);
    let out = outbox.resolve(1, false, Vec::new());
    // Then
    assert_eq!(out, [start(1.0), phrase("restart dictation"), end(2.0)]);
}

#[test]
fn should_give_the_spotted_phrase_kept_the_timing_of_the_confirmation_that_heard_it() {
    // Given
    let timing = Timing { asked_at: 2.5, took_ms: 200 };
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(1, vec![start(1.0), phrase("start dictation"), end(2.0)]);
    // When
    outbox.keep_only_phrase(1, "start dictation", Some(timing));
    let out = outbox.resolve(1, false, vec![timed_phrase("start dictation", timing)]);
    // Then
    assert_eq!(
        out,
        [
            start(1.0),
            timed_phrase("start dictation", timing),
            end(2.0),
            timed_phrase("start dictation", timing),
        ]
    );
}

/// A segment held under confirmation 1 whose wait moved to confirmation 2, asked after
/// `between` ran.
fn with_a_segment_moved_to_a_later_confirmation(between: impl FnOnce(&mut Outbox)) -> Outbox {
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(1, vec![start(1.0), end(2.0)]);
    between(&mut outbox);
    outbox.asked(2, false);
    outbox.transfer(1, 2);
    outbox
}

#[test]
fn should_keep_a_segment_whose_wait_moved_to_another_confirmation_until_that_one() {
    // Given
    let mut outbox = with_a_segment_moved_to_a_later_confirmation(|_| {});
    // When
    let first = outbox.resolve(1, false, Vec::new());
    let second = outbox.resolve(2, false, vec![phrase("press enter")]);
    // Then
    assert!(first.is_empty());
    assert_eq!(second, [start(1.0), end(2.0), phrase("press enter")]);
}

#[test]
fn should_be_busy_while_a_check_waits() {
    // When
    let mut outbox = Outbox::default();
    outbox.asked(1, true);
    // Then
    assert!(outbox.check_busy());
}

#[test]
fn should_answer_a_sync_at_once_when_nothing_waits() {
    assert_eq!(Outbox::default().sync(7), [ServerEvent::Synced { id: 7 }]);
}

#[test]
fn should_answer_a_sync_only_after_the_confirmations_asked_for_before_it() {
    // Given
    let mut outbox = Outbox::default();
    outbox.asked(1, false);
    outbox.hold(1, vec![end(2.0)]);
    // When
    let early = outbox.sync(7);
    outbox.asked(2, true);
    let out = outbox.resolve(1, false, Vec::new());
    // Then
    assert!(early.is_empty());
    assert_eq!(out, [end(2.0), ServerEvent::Synced { id: 7 }]);
}

#[test]
fn should_keep_a_sync_waiting_for_a_segment_whose_wait_moved_to_a_confirmation_asked_after_it() {
    // Given
    let mut early = Vec::new();
    let mut outbox = with_a_segment_moved_to_a_later_confirmation(|o| early = o.sync(7));
    // When
    let first = outbox.resolve(1, false, Vec::new());
    let second = outbox.resolve(2, false, Vec::new());
    // Then
    assert!(early.is_empty());
    assert!(
        first.is_empty(),
        "the sync was answered before the segment: {first:?}"
    );
    assert_eq!(
        second,
        [start(1.0), end(2.0), ServerEvent::Synced { id: 7 }]
    );
}
