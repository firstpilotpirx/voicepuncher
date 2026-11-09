use super::*;

mod at_once;
mod chain_tail;
mod reported;

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
    Moment::Decoding { now, pause: None, synced: false }
}

#[test]
fn should_report_a_doubled_command_at_once_ending_at_its_first_word() {
    // Given "stop stop", the first word ending at 1.25 s and the second at 1.55 s
    let words = said(1.0, "stop stop");
    // When
    let found = decide(&dictating(), &words, decoding(1.6), &Reported::default());
    // Then the command ends with its first word: a recognizer that emits late times the
    // second word inside the speech after it
    assert_stop_with_speech_after(&found.expect("a doubled stop"));
}

#[test]
fn should_end_a_doubled_command_written_with_a_capital_and_a_comma_at_its_first_word() {
    // Given "Restart, restart" as the English recognizer writes it, the first word ending at
    // 1.25 s and the second at 1.55 s
    let words = said(1.0, "Restart, restart");
    let matcher = PhraseMatcher::new(&["restart dictation".into(), "stop dictation".into()]);
    // When
    let found = decide(&matcher, &words, decoding(1.6), &Reported::default());
    // Then the words count as the same word, whatever their case and punctuation
    let found = found.expect("a doubled restart");
    assert_eq!(found.commands, ["restart dictation"]);
    assert!((found.end - 1.25).abs() < 1e-4, "{}", found.end);
}

#[test]
fn should_wait_for_a_one_word_form_to_settle() {
    // Given "stop" just heard, ending at 1.25 s
    let words = said(1.0, "stop");
    // When decoded 0.05 s past it, and when decoded 0.25 s past it with the gate's pause
    // begun right after it
    let early = decide(&dictating(), &words, decoding(1.3), &Reported::default());
    let paused = Moment::Decoding { now: 1.5, pause: Some(1.28), synced: false };
    let settled = decide(&dictating(), &words, paused, &Reported::default());
    // Then
    assert_eq!(early, None);
    assert_eq!(
        settled.expect("a settled stop").commands,
        ["stop dictation"]
    );
}

#[test]
fn should_report_a_command_ending_a_sentence_at_the_gates_pause() {
    // Given "this is done stop", its "stop" ending at 2.15 s, the gate silent from 2.2 s
    let words = said(1.0, "this is done stop");
    // When
    let early = decide(
        &dictating(),
        &words,
        paused_at(2.18, 2.2),
        &Reported::default(),
    );
    let paused = decide(
        &dictating(),
        &words,
        paused_at(2.36, 2.2),
        &Reported::default(),
    );
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
        &Reported::default(),
    );
    // Then
    let found = found.expect("a start at the head");
    assert_eq!(found.commands, ["start dictation"]);
    assert_eq!(found.placement, Placement::Head);
}

#[test]
fn should_report_an_early_start_before_speech_follows() {
    // When
    let found = decide(
        &standby(),
        &said(1.0, "start"),
        decoding(1.26),
        &Reported::default(),
    );
    // Then
    assert_eq!(found.expect("an early start").placement, Placement::Head);
}

#[test]
fn should_report_nothing_without_words() {
    // When
    let found = decide(&standby(), &[], Moment::Closing, &Reported::default());
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
    Moment::Decoding { now, pause: Some(pause), synced: false }
}

#[test]
fn should_report_a_lone_stop_once_decoded_up_to_the_gates_pause() {
    // Given "stop" ending at 1.25 s, the gate silent from 1.3 s
    let words = said(1.0, "stop");
    // When
    let found = decide(
        &dictating(),
        &words,
        paused_at(1.32, 1.3),
        &Reported::default(),
    );
    // Then
    assert_eq!(found.expect("a lone stop").commands, ["stop dictation"]);
}

#[test]
fn should_wait_for_the_recognizer_to_decode_up_to_the_pause() {
    // When
    let found = decide(
        &dictating(),
        &said(1.0, "stop"),
        paused_at(1.28, 1.3),
        &Reported::default(),
    );
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_wait_for_a_command_ending_a_sentence_to_settle_past_the_pause() {
    // Given "let's start" heard so far, its "start" ending at 1.55 s, the gate silent from
    // 1.55 s: the recognizer may still revise it to "restart"
    let words = said(1.0, "let's start");
    // When
    let found = decide(
        &standby(),
        &words,
        paused_at(1.6, 1.55),
        &Reported::default(),
    );
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
    let paused = decide(&matcher, &words, paused_at(2.0, 1.6), &Reported::default());
    let closing = decide(&matcher, &words, Moment::Closing, &Reported::default());
    // Then
    assert_eq!(paused, None);
    assert_eq!(
        closing.expect("the close decides").commands,
        ["start dictation"]
    );
}

#[test]
fn should_take_each_command_of_an_utterance_said_with_a_pause_between_them() {
    // Given "press enter", a 0.75 s pause the gate missed, and "press enter" again
    let words = [said(1.0, "press enter"), said(2.3, "press enter")].concat();
    // When
    let first =
        decide(&standby(), &words, Moment::Closing, &Reported::default()).expect("the first");
    let reported = Reported { until: first.end, commands: first.commands.clone() };
    let second = decide(&standby(), &words, Moment::Closing, &reported).expect("the second");
    // Then
    assert!((first.end - 1.55).abs() < 1e-6, "{}", first.end);
    assert!((second.end - 2.85).abs() < 1e-6, "{}", second.end);
}

/// Asserts `found` is a stop with speech after it, ending at 1.25 s.
fn assert_stop_with_speech_after(found: &Finding) {
    assert_eq!(found.commands, ["stop dictation"]);
    assert_eq!(found.placement, Placement::Anywhere);
    assert!((found.end - 1.25).abs() < 1e-4, "{}", found.end);
}
