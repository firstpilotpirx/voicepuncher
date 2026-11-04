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
fn should_wait_for_a_one_word_form_to_settle() {
    // Given "stop" just heard, ending at 1.25 s
    let words = said(1.0, "stop");
    // When decoded 0.05 s past it, and when decoded 0.25 s past it with the gate's pause
    // begun right after it
    let early = decide(&dictating(), &words, decoding(1.3), 0.0);
    let paused = Moment::Decoding { now: 1.5, pause: Some(1.28) };
    let settled = decide(&dictating(), &words, paused, 0.0);
    // Then
    assert_eq!(early, None);
    assert_eq!(
        settled.expect("a settled stop").commands,
        ["stop dictation"]
    );
}

#[test]
fn should_not_settle_a_lone_word_while_the_gate_hears_speech() {
    // Given "stop" (the user's "чтобы" of history entry 684) ending at 0.918 s, decoded 0.2 s
    // past it
    let words = vec![TimedWord { text: "stop".into(), start: 0.818, end: 0.918 }];
    // When the gate still hears speech, and when its pause began right after the word
    let speech_goes_on = Moment::Decoding { now: 1.118, pause: None };
    let paused = Moment::Decoding { now: 1.118, pause: Some(1.0) };
    let unsettled = decide(&dictating(), &words, speech_goes_on, 0.0);
    let settled = decide(&dictating(), &words, paused, 0.0);
    // Then
    assert_eq!(unsettled, None);
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
    // Given "press enter" reported up to 1.55 s, then "stop" ending at 1.85 s, the gate's
    // pause begun right after it
    let words = said(1.0, "press enter stop");
    // When
    let paused = Moment::Decoding { now: 2.1, pause: Some(1.9) };
    let next = decide(&dictating(), &words, paused, 1.55);
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

#[test]
fn should_take_a_start_timed_late_after_a_restart_for_its_end() {
    // Given "restart" reported at 3.84–4.46 s and "start" timed by the other recognizer at 5.1 s
    let restart = Finding {
        commands: vec!["restart dictation".into()],
        start: 3.84,
        end: 4.46,
        placement: Placement::Tail,
    };
    let start = Finding {
        commands: vec!["start dictation".into()],
        start: 5.1,
        end: 5.1,
        placement: Placement::Head,
    };
    // When / Then
    assert!(doubles(Some(&restart), &start));
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

#[test]
fn should_wait_for_the_close_when_a_sentence_ends_in_the_end_of_another_command() {
    // Given "I start" heard for a quiet "restart" while both are listened for, the gate
    // silent from 1.6 s
    let matcher = PhraseMatcher::new(&["start dictation".into(), "restart dictation".into()]);
    let words = said(1.0, "I start");
    // When
    let paused = decide(&matcher, &words, paused_at(2.0, 1.6), 0.0);
    let closing = decide(&matcher, &words, Moment::Closing, 0.0);
    // Then
    assert_eq!(paused, None);
    assert_eq!(
        closing.expect("the close decides").commands,
        ["start dictation"]
    );
}

#[test]
fn should_wait_longer_for_a_one_word_form_when_the_gate_heard_speech_go_on_after_it() {
    // Given "stop" ending at 1.25 s, the gate's pause beginning only at 1.91 s ("stop the car")
    let words = said(1.0, "stop");
    // When
    let early = decide(&dictating(), &words, paused_at(1.55, 1.91), 0.0);
    // Then
    assert_eq!(early, None);
}

#[test]
fn should_wait_for_the_pause_when_the_gate_heard_speech_go_on_after_a_one_word_form() {
    // Given "stop" ending at 1.25 s, decoded to 1.75 s, the gate's pause beginning at 2.1 s
    let words = said(1.0, "stop");
    // When
    let found = decide(&dictating(), &words, paused_at(1.75, 2.1), 0.0);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_take_each_command_of_an_utterance_said_with_a_pause_between_them() {
    // Given "press enter", a 0.75 s pause the gate missed, and "press enter" again
    let words = [said(1.0, "press enter"), said(2.3, "press enter")].concat();
    // When
    let first = decide(&standby(), &words, Moment::Closing, 0.0).expect("the first");
    let second = decide(&standby(), &words, Moment::Closing, first.end).expect("the second");
    // Then
    assert!((first.end - 1.55).abs() < 1e-6, "{}", first.end);
    assert!((second.end - 2.85).abs() < 1e-6, "{}", second.end);
}

#[test]
fn should_take_no_command_followed_by_speech_without_a_pause() {
    // When
    let found = decide(
        &standby(),
        &said(1.0, "press enter the door"),
        Moment::Closing,
        0.0,
    );
    // Then
    assert_eq!(found, None);
}
