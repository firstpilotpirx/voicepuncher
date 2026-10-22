use super::*;

fn spotted(start: f32, end: f32) -> Spotted {
    Spotted { phrase: "press enter".into(), start, end }
}

fn phrase(start: f32, end: f32) -> ServerEvent {
    ServerEvent::Phrase { phrase: "press enter".into(), start, end, timing: None }
}

#[test]
fn should_report_a_phrase_spotted_during_its_segment_between_start_and_end() {
    // Given
    let mut orderer = EventOrderer::default();
    // When
    let first = orderer.phrase(spotted(1.1, 1.8));
    let then = orderer.segment(Segment { start: 1.0, end: 1.9 });
    // Then
    assert!(first.is_empty());
    assert_eq!(
        then,
        [
            ServerEvent::SpeechStart { t: 1.0 },
            phrase(1.1, 1.8),
            ServerEvent::SpeechEnd { t: 1.9 }
        ]
    );
}

#[test]
fn should_report_a_phrase_spotted_after_its_segment_ended_at_once() {
    // Given
    let mut orderer = EventOrderer::default();
    orderer.segment(Segment { start: 1.0, end: 1.9 });
    // When
    let events = orderer.phrase(spotted(1.1, 1.8));
    // Then
    assert_eq!(events, [phrase(1.1, 1.8)]);
}

#[test]
fn should_keep_a_phrase_of_a_later_segment_waiting() {
    // Given
    let mut orderer = EventOrderer::default();
    orderer.phrase(spotted(5.1, 5.8));
    // When
    let first = orderer.segment(Segment { start: 1.0, end: 1.9 });
    let later = orderer.segment(Segment { start: 5.0, end: 5.9 });
    // Then
    assert!(!first.contains(&phrase(5.1, 5.8)));
    assert!(later.contains(&phrase(5.1, 5.8)));
}

#[test]
fn should_drop_a_phrase_from_before_a_segment() {
    // Given
    let mut orderer = EventOrderer::default();
    orderer.phrase(spotted(0.1, 0.5));
    // When
    orderer.segment(Segment { start: 3.0, end: 3.9 });
    let later = orderer.segment(Segment { start: 5.0, end: 5.9 });
    // Then
    assert!(!later.contains(&phrase(0.1, 0.5)));
}

#[test]
fn should_keep_a_phrase_whose_start_was_stretched_before_its_segment() {
    // Given the spotter placed the phrase start half a second before the speech began
    let mut orderer = EventOrderer::default();
    orderer.phrase(spotted(25.5, 26.75));
    // When
    let events = orderer.segment(Segment { start: 26.0, end: 27.05 });
    // Then
    assert!(events.contains(&phrase(25.5, 26.75)), "{events:?}");
}

#[test]
fn should_give_up_waiting_for_the_segment_of_a_phrase_the_detector_missed() {
    // Given
    let mut orderer = EventOrderer::default();
    orderer.phrase(spotted(10.0, 11.0));
    // When
    let early = orderer.unheld(11.5);
    let late = orderer.unheld(12.5);
    // Then
    assert!(early.is_empty());
    assert_eq!(late, [spotted(10.0, 11.0)]);
    assert!(orderer.unheld(13.0).is_empty());
}
