//! Chains and checks of a running utterance.

use super::*;

#[test]
fn should_carry_out_the_rest_of_a_chain_a_check_heard_only_in_part() {
    let _shared = engine();
    // Given a running utterance whose check is cut off inside its last command
    let hear = HearsTheWordsInTheClip::said(&["never mind", "cancel", "enter", "start"], 0.12);
    let engine = own_engine(None).hearing(hear.clone());
    let mut session = listening(&engine, &DICTATING_PHRASES);
    // When the whole utterance is heard once it ends
    let events = fed_answering_at_once(&mut session, &hear.stream);
    // Then
    assert!(
        hear.cut_off.lock().unwrap().contains(&"start"),
        "no check was cut off inside \"start\": {:?}",
        hear.cut_off.lock().unwrap()
    );
    assert_eq!(
        reported_phrases(&events),
        ["cancel dictation", "press enter", "start dictation"],
        "{events:?}"
    );
}

#[test]
fn should_report_start_start_followed_at_once_by_speech_soon_after_its_end() {
    let _shared = engine();
    // Given "start start" and the dictation right after it, in one utterance
    let hear = HearsTheWordsInTheClip::said(&["start", "start", "we meet on Friday at ten"], 0.12);
    let engine = own_engine(None).hearing(hear.clone());
    let mut session = listening(&engine, &STANDBY_PHRASES);
    // When
    let reported = reported_at(&mut session, &hear.stream);
    // Then
    let command_end = hear.words[1].1.end;
    let [(commands, at)] = reported.as_slice() else {
        panic!("one report of start expected: {reported:?}");
    };
    assert_eq!(commands, &["start dictation"]);
    let late = (*at as f32 - command_end as f32) / SAMPLE_RATE as f32;
    assert!(
        late <= START_REPORTED_WITHIN_SECS,
        "start start reported {late:.2} s after its end"
    );
}

/// How long after the end of its words a doubled command inside speech may be reported,
/// every confirmation answered at once: well before the checks of the running utterance.
const DOUBLED_REPORTED_WITHIN_SECS: f32 = 0.3;

#[test]
fn should_report_stop_stop_inside_speech_soon_after_its_end() {
    let _shared = engine();
    // Given "stop stop" between two sentences of a dictation, in one utterance
    let hear = HearsTheWordsInTheClip::said(
        &[
            "we meet on Friday at ten",
            "stop",
            "stop",
            "the coffee machine is broken",
        ],
        0.12,
    );
    let engine = own_engine(None).hearing(hear.clone());
    let mut session = listening(&engine, &DICTATING_PHRASES);
    // When
    let reported = reported_at(&mut session, &hear.stream);
    // Then
    let command_end = hear.words[2].1.end;
    let Some((commands, at)) = reported.first() else {
        panic!("stop stop was never reported");
    };
    assert_eq!(commands, &["stop dictation"]);
    let late = (*at as f32 - command_end as f32) / SAMPLE_RATE as f32;
    assert!(
        late <= DOUBLED_REPORTED_WITHIN_SECS,
        "stop stop reported {late:.2} s after its end"
    );
}

#[test]
fn should_check_a_running_utterance_soon_after_a_short_start_and_never_report_its_trigger() {
    let _shared = engine();
    // Given "start" and the dictation right after it, in one utterance
    let hear = HearsTheWordsInTheClip::said(&["start", "we meet on Friday at ten"], 0.05);
    let command_end = hear.words[0].1.end;
    // When
    let (delay, events) = report_delay(&hear, &STANDBY_PHRASES, command_end);
    // Then
    assert!(
        delay.is_some_and(|d| d <= SPOTTED_CHECK_WITHIN_SECS),
        "start reported {delay:?} s after its end: {events:?}"
    );
    assert_eq!(trigger_phrases(&events), Vec::<String>::new());
}

#[test]
fn should_not_start_on_start_stop_said_in_standby() {
    let _shared = engine();
    // Given "start stop" said in one breath while no dictation runs
    let hear = HearsTheWordsInTheClip::said(&["start", "stop"], 0.05);
    let command_end = hear.words[0].1.end;
    // When
    let (delay, events) = report_delay(&hear, &STANDBY_PHRASES, command_end);
    // Then
    assert_eq!(delay, None, "a command reported: {events:?}");
}

#[test]
fn should_not_start_on_start_inside_a_sentence_said_in_standby() {
    let _shared = engine();
    // Given a sentence with "start" inside it while no dictation runs
    let hear = HearsTheWordsInTheClip::said(&["we", "start", "at nine tomorrow"], 0.05);
    let command_end = hear.words[1].1.end;
    // When
    let (delay, events) = report_delay(&hear, &STANDBY_PHRASES, command_end);
    // Then
    assert_eq!(delay, None, "a command reported: {events:?}");
}

#[test]
fn should_check_a_running_utterance_soon_after_a_short_restart_and_never_report_its_trigger() {
    let _shared = engine();
    // Given a dictated sentence, "restart restart" and more speech, in one utterance
    let hear = HearsTheWordsInTheClip::said(
        &[
            "we meet on Friday",
            "restart",
            "restart",
            "and then more words",
        ],
        0.05,
    );
    let command_end = hear.words[2].1.end;
    // When
    let (delay, events) = report_delay(&hear, &DICTATING_PHRASES, command_end);
    // Then
    assert!(
        delay.is_some_and(|d| d <= START_REPORTED_WITHIN_SECS),
        "restart restart reported {delay:?} s after its end: {events:?}"
    );
    assert_eq!(trigger_phrases(&events), Vec::<String>::new());
}

#[test]
fn should_report_nothing_of_a_check_answered_after_its_utterances_segment() {
    let _shared = engine();
    // Given "start dictation please" said alone, its head check answered late
    let words = HearsTheWordsInTheClip::said(&["start", "dictation", "please"], 0.15);
    let stream = words.stream.clone();
    let hear = Arc::new(SlowFirstClip { words, clips: Mutex::new(0) });
    let engine = own_engine(None).hearing(hear.clone());
    let mut session = listening(&engine, &STANDBY_PHRASES);
    // When
    let events = fed_in_real_time(&mut session, &stream);
    // Then
    assert!(
        *hear.clips.lock().unwrap() > 1,
        "no check was asked: {events:?}"
    );
    let segment_end = events
        .iter()
        .position(|e| matches!(e, ServerEvent::SpeechEnd { .. }))
        .expect("the utterance ended");
    assert!(
        !events[segment_end..]
            .iter()
            .any(|e| matches!(e, ServerEvent::Command { .. })),
        "{events:?}"
    );
}

#[test]
fn should_check_the_start_of_a_running_utterance_soon_though_no_trigger_was_spotted() {
    let _shared = engine();
    // Given "start" the keyword spotter does not know, as it misses most of the user's,
    // and the dictation right after it, in one utterance
    let hear = HearsTheWordsInTheClip::said_as(
        &[
            ("start", "mark"),
            ("we meet on Friday at ten", "we meet on Friday at ten"),
        ],
        0.05,
    );
    let command_end = hear.words[0].1.end;
    // When
    let (delay, events) = report_delay(&hear, &STANDBY_PHRASES, command_end);
    // Then
    assert!(
        delay.is_some_and(|d| d <= START_REPORTED_WITHIN_SECS),
        "start reported {delay:?} s after its end: {events:?}"
    );
}
