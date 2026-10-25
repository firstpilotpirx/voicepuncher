//! Utterances confirmed, ended and heard again by speech recognition.

use super::*;

#[test]
fn should_confirm_an_utterance_with_what_the_speech_recognition_service_heard() {
    let _shared = engine();
    // Given
    let engine = own_engine(None).hearing(Arc::new(HearsPressEnter));
    // When
    let events = events_for(&engine, NOT_A_COMMAND);
    // Then
    assert_eq!(reported_phrases(&events), ["press enter"], "{events:?}");
}

#[test]
fn should_confirm_a_command_said_alone_by_the_second_attempt() {
    let _shared = engine();
    // Given
    let engine = own_engine(Some(speech_recognition_hanging_first(1)));
    // When
    let events = events_for(&engine, NOT_A_COMMAND);
    // Then
    assert_eq!(reported_phrases(&events), ["press enter"], "{events:?}");
}

#[test]
fn should_spot_a_command_after_a_hung_confirmation() {
    let _shared = engine();
    // Given
    let engine = own_engine(Some(speech_recognition_hanging_first(2)));
    let mut session = listening(&engine, &ALL_PHRASES);
    let mut audio = spoken(NOT_A_COMMAND);
    audio.extend(spoken("press enter"));
    // When
    let events = fed(&mut session, &audio);
    // Then
    assert!(
        reported_phrases(&events).contains(&"press enter".to_owned()),
        "{events:?}"
    );
}

#[test]
fn should_keep_the_checks_of_a_long_utterance_few_while_one_waits_for_a_slow_service() {
    let _shared = engine();
    // Given
    let hear = Arc::new(SlowHear(std::sync::atomic::AtomicU32::new(0)));
    let mut cfg = engine_cfg(None);
    cfg.confirmation_deadline = PATIENT_DEADLINE;
    let engine = Engine::load(&cfg)
        .expect("models load")
        .hearing(hear.clone());
    // When
    let events = events_for(&engine, LONG_UTTERANCE);
    // Then
    let clips = hear.0.load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        clips <= MOST_CLIPS_WHILE_SLOW,
        "{clips} clips were sent for one utterance while the service was slow: {events:?}"
    );
}

#[test]
fn should_leave_the_keyword_spotted_phrase_to_decide_when_its_segment_confirmation_is_given_up() {
    let _shared = engine();
    // Given
    let engine = own_engine(None).hearing(Arc::new(NeverInTime));
    // When
    let events = events_for(&engine, "press enter");
    // Then
    assert_eq!(spotted(&events), ["press enter"], "{events:?}");
}

#[test]
fn should_end_an_utterance_at_a_short_pause_before_more_speech() {
    let engine = engine();
    // Given
    let command = sound_only(&common::spoken_by(None, "Restart"));
    // When
    let (ends, sentence_start, events) = ends_before_sentence(&engine, &command, SHORT_PAUSE_SECS);
    // Then
    assert!(
            ends.first().is_some_and(|end| *end < sentence_start),
            "the command's utterance did not end before the sentence at {sentence_start:.2}s: {events:?}"
        );
}

#[test]
fn should_hear_a_command_ending_a_long_utterance_in_its_last_seconds() {
    let _shared = engine();
    // Given
    let engine = own_engine(None).hearing(Arc::new(HearsTheCommandOnlyInTheTail));
    // When
    let events = events_for(&engine, SENTENCE_ENDING_IN_ENTER);
    // Then
    assert_eq!(reported_phrases(&events), ["press enter"], "{events:?}");
}

#[test]
fn should_take_a_near_miss_of_restart_as_restart_when_its_second_hearing_says_restart() {
    let _shared = engine();
    // Given
    let first = "Let's start.";
    // When
    let (events, clips) = restart_heard_after(first);
    // Then
    assert_eq!(
        reported_phrases(&events),
        ["restart dictation"],
        "{events:?}"
    );
    assert_eq!(clips, 2);
}

#[test]
fn should_hear_a_restart_the_spotter_heard_again_when_its_segment_is_heard_as_another_word() {
    let _shared = engine();
    // Given "restart" said alone, heard first as another word
    let first = "Restocked.";
    // When
    let (events, clips) = restart_heard_after(first);
    // Then
    assert_eq!(
        reported_phrases(&events),
        ["restart dictation"],
        "{events:?}"
    );
    assert_eq!(clips, 2);
}

#[test]
fn should_place_the_restart_of_a_second_hearing_at_the_start_of_its_speech() {
    let _shared = engine();
    // Given
    let hear = HearsInTurn::new(&["Let's start.", "Restart."]);
    // When
    let events = heard_alone(hear, &DICTATING_PHRASES, "Restart");
    // Then
    let (segment, phrase) = segment_and_phrase(&events);
    assert!(
        (phrase.0 - segment.0).abs() < LEAD_TOLERANCE_SECS,
        "lead: segment {segment:?} phrase {phrase:?}"
    );
    assert!(
        phrase.1 <= segment.1 + TAIL_TOLERANCE_SECS,
        "tail: segment {segment:?} phrase {phrase:?}"
    );
}

#[test]
fn should_take_no_command_when_the_second_hearing_of_a_near_miss_says_it_again() {
    let _shared = engine();
    // Given
    let hear = HearsInTurn::new(&["Let's start.", "Let's start."]);
    // When
    let events = heard_alone(hear.clone(), &DICTATING_PHRASES, "Let's start");
    // Then
    assert!(reported_phrases(&events).is_empty(), "{events:?}");
    assert_eq!(hear.clips(), 2);
}

#[test]
fn should_take_no_restart_from_a_second_hearing_when_restart_is_not_selected() {
    let _shared = engine();
    // Given
    let standby = ["start dictation", "press enter", "clear all text"];
    let hear = HearsInTurn::new(&["Let's start.", "Restart."]);
    // When
    let events = heard_alone(hear, &standby, "Let's start");
    // Then
    assert!(
        !reported_phrases(&events)
            .iter()
            .any(|p| p == "restart dictation"),
        "{events:?}"
    );
}

/// The events of "restart" said alone and heard first as `first`, then as "Restart.", and
/// the count of the clips heard.
fn restart_heard_after(first: &'static str) -> (Vec<ServerEvent>, usize) {
    let hear = HearsInTurn::new(&[first, "Restart."]);
    let events = heard_alone(hear.clone(), &DICTATING_PHRASES, "Restart");
    (events, hear.clips())
}
