use super::*;

fn standby() -> PhraseMatcher {
    PhraseMatcher::new(&[
        "start dictation".into(),
        "press enter".into(),
        "clear all text".into(),
        "paste text".into(),
    ])
}

fn dictating() -> PhraseMatcher {
    PhraseMatcher::new(&[
        "stop dictation".into(),
        "cancel dictation".into(),
        "press enter".into(),
    ])
}

/// Words said one after another, 0.3 s each from `from`.
fn said(from: f32, text: &str) -> Vec<TimedWord> {
    text.split(' ')
        .enumerate()
        .map(|(i, w)| {
            let start = from + 0.3 * i as f32;
            TimedWord { text: w.into(), start, end: start + 0.25 }
        })
        .collect()
}

fn decoding(now: f32) -> Moment {
    Moment::Decoding { now, pause: None }
}

#[test]
fn should_report_a_doubled_command_at_once() {
    // When
    let found = decide(&dictating(), &said(1.0, "stop stop"), decoding(1.6), 0.0);
    // Then
    let found = found.expect("a doubled stop");
    assert_eq!(found.commands, ["stop dictation"]);
    assert_eq!(found.placement, Placement::Anywhere);
    assert!((found.end - 1.55).abs() < 1e-4, "{}", found.end);
}

#[test]
fn should_report_a_two_word_form_at_once() {
    // When
    let found = decide(&standby(), &said(1.0, "press enter"), decoding(1.56), 0.0);
    // Then
    let found = found.expect("press enter");
    assert_eq!(found.commands, ["press enter"]);
    assert_eq!(found.placement, Placement::Tail);
    assert!((found.start - 1.0).abs() < 1e-4, "{}", found.start);
}

#[test]
fn should_wait_for_a_one_word_form_to_settle() {
    // Given "stop" just heard, ending at 1.25 s
    let words = said(1.0, "stop");
    // When
    let early = decide(&dictating(), &words, decoding(1.3), 0.0);
    let settled = decide(&dictating(), &words, decoding(1.5), 0.0);
    // Then
    assert_eq!(early, None);
    assert_eq!(
        settled.expect("a settled stop").commands,
        ["stop dictation"]
    );
}

#[test]
fn should_not_take_the_first_word_of_a_sentence_for_a_command() {
    // When
    let found = decide(&dictating(), &said(1.0, "stop the car"), decoding(2.0), 0.0);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_report_a_command_ending_speech_only_when_the_utterance_closes() {
    // Given
    let words = said(1.0, "this is done stop");
    // When
    let running = decide(&dictating(), &words, decoding(3.0), 0.0);
    let closing = decide(&dictating(), &words, Moment::Closing, 0.0);
    // Then
    assert_eq!(running, None);
    let found = closing.expect("a stop ending the utterance");
    assert_eq!(found.commands, ["stop dictation"]);
    assert_eq!(found.placement, Placement::Tail);
}

#[test]
fn should_report_a_command_ending_a_sentence_at_the_gates_pause() {
    // Given "this is done stop", its "stop" ending at 2.15 s, the gate silent from 2.2 s
    let words = said(1.0, "this is done stop");
    // When
    let early = decide(&dictating(), &words, paused_at(2.18, 2.2), 0.0);
    let paused = decide(&dictating(), &words, paused_at(2.36, 2.2), 0.0);
    // Then
    assert_eq!(early, None);
    let found = paused.expect("a stop ending the sentence");
    assert_eq!(found.commands, ["stop dictation"]);
    assert_eq!(found.placement, Placement::Tail);
}

#[test]
fn should_report_a_start_opening_speech_at_its_head() {
    // When
    let found = decide(
        &standby(),
        &said(1.0, "start hello world"),
        decoding(2.0),
        0.0,
    );
    // Then
    let found = found.expect("a start at the head");
    assert_eq!(found.commands, ["start dictation"]);
    assert_eq!(found.placement, Placement::Head);
}

#[test]
fn should_report_an_early_start_before_speech_follows() {
    // When
    let found = decide(&standby(), &said(1.0, "start"), decoding(1.26), 0.0);
    // Then
    assert_eq!(found.expect("an early start").placement, Placement::Head);
}

#[test]
fn should_skip_words_said_before_the_last_report() {
    // Given "stop stop" reported up to 1.55 s
    let words = said(1.0, "stop stop");
    // When
    let again = decide(&dictating(), &words, Moment::Closing, 1.55);
    // Then
    assert_eq!(again, None);
}

#[test]
fn should_report_the_next_command_said_after_the_last_report() {
    // Given "press enter" reported up to 1.55 s, then "stop"
    let words = said(1.0, "press enter stop");
    // When
    let next = decide(&dictating(), &words, decoding(2.1), 1.55);
    // Then
    assert_eq!(
        next.expect("the stop after it").commands,
        ["stop dictation"]
    );
}

#[test]
fn should_report_nothing_without_words() {
    // When
    let found = decide(&standby(), &[], Moment::Closing, 0.0);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_wait_for_a_command_heard_with_a_word_after_it_to_settle() {
    // Given "stop the" heard so far of "stop the car"
    let words = said(1.0, "stop the");
    // When
    let found = decide(&dictating(), &words, decoding(1.6), 0.0);
    // Then
    assert_eq!(found, None);
}

fn found_at(start: f32, end: f32) -> Finding {
    Finding { commands: vec!["press enter".into()], start, end, placement: Placement::Tail }
}

#[test]
fn should_take_the_same_command_right_after_a_report_for_its_double() {
    // Given "enter" reported at 1.24–1.34 s
    let last = found_at(1.24, 1.34);
    // When / Then
    assert!(doubles(Some(&last), &found_at(1.64, 1.98)));
    assert!(!doubles(Some(&last), &found_at(2.5, 2.8)));
    assert!(!doubles(None, &found_at(1.64, 1.98)));
}

fn paused_at(now: f32, pause: f32) -> Moment {
    Moment::Decoding { now, pause: Some(pause) }
}

#[test]
fn should_report_a_lone_stop_once_decoded_up_to_the_gates_pause() {
    // Given "stop" ending at 1.25 s, the gate silent from 1.3 s
    let words = said(1.0, "stop");
    // When
    let found = decide(&dictating(), &words, paused_at(1.32, 1.3), 0.0);
    // Then
    assert_eq!(found.expect("a lone stop").commands, ["stop dictation"]);
}

#[test]
fn should_wait_for_the_recognizer_to_decode_up_to_the_pause() {
    // When
    let found = decide(&dictating(), &said(1.0, "stop"), paused_at(1.28, 1.3), 0.0);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_not_settle_a_lone_stop_while_the_gate_hears_speech() {
    // When "stop" is decoded but speech goes on
    let found = decide(&dictating(), &said(1.0, "stop"), decoding(1.35), 0.0);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_wait_for_a_command_ending_a_sentence_to_settle_past_the_pause() {
    // Given "let's start" heard so far, its "start" ending at 1.55 s, the gate silent from
    // 1.55 s: the recognizer may still revise it to "restart"
    let words = said(1.0, "let's start");
    // When
    let found = decide(&standby(), &words, paused_at(1.6, 1.55), 0.0);
    // Then
    assert_eq!(found, None);
}
