//! A command word counts wherever it is said, once decoded past it: no pause is awaited.

use super::*;

#[test]
fn should_report_a_lone_word_while_the_gate_still_hears_speech() {
    // Given "stop" ending at 1.25 s, decoded 0.2 s past it, the gate hearing speech
    let words = said(1.0, "stop");
    // When
    let found = decide(&dictating(), &words, decoding(1.45), &Reported::default());
    // Then
    let found = found.expect("a stop decoded past");
    assert_eq!(found.commands, ["stop dictation"]);
    assert_eq!(found.placement, Placement::Tail);
}

#[test]
fn should_not_report_a_word_decoded_less_than_its_settle_time_past() {
    // When "stop", ending at 1.25 s, is decoded only 0.1 s past: its draft may still grow
    let found = decide(
        &dictating(),
        &said(1.0, "stop"),
        decoding(1.35),
        &Reported::default(),
    );
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_report_a_command_ending_speech_without_a_pause() {
    // Given "this is done stop", its "stop" ending at 2.15 s, the gate hearing speech
    let words = said(1.0, "this is done stop");
    // When
    let found = decide(&dictating(), &words, decoding(2.4), &Reported::default());
    // Then
    let found = found.expect("a stop ending speech");
    assert_eq!(found.commands, ["stop dictation"]);
    assert_eq!(found.placement, Placement::Tail);
    assert!((found.start - 1.9).abs() < 1e-4, "{}", found.start);
}

#[test]
fn should_report_a_command_opening_a_sentence_once_a_word_follows() {
    // Given "stop the" heard so far of "stop the car", decoded just past "the"
    let words = said(1.0, "stop the");
    // When
    let found = decide(&dictating(), &words, decoding(1.56), &Reported::default());
    // Then the word after it shows "stop" is decoded: it counts, with speech after it
    assert_stop_with_speech_after(&found.expect("a stop opening a sentence"));
}

#[test]
fn should_report_a_command_inside_a_sentence() {
    // When
    let found = decide(
        &standby(),
        &said(1.0, "now press enter the door"),
        decoding(2.6),
        &Reported::default(),
    );
    // Then
    let found = found.expect("an enter inside a sentence");
    assert_eq!(found.commands, ["press enter"]);
    assert_eq!(found.placement, Placement::Anywhere);
}

#[test]
fn should_report_a_paste_word_inside_a_sentence() {
    // When
    let found = decide(
        &standby(),
        &said(1.0, "then paste it here"),
        decoding(2.2),
        &Reported::default(),
    );
    // Then
    assert_eq!(found.expect("a paste").commands, ["paste text"]);
}

#[test]
fn should_not_report_a_command_the_state_does_not_accept() {
    // When "start" is said inside a dictation
    let found = decide(
        &dictating(),
        &said(1.0, "we start now"),
        decoding(2.0),
        &Reported::default(),
    );
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_keep_waiting_for_a_last_word_that_may_grow_into_a_longer_command_word() {
    // Given "I start" heard while "restart" is listened for too, decoded 0.4 s past it
    let matcher = PhraseMatcher::new(&["start dictation".into(), "restart dictation".into()]);
    // When
    let found = decide(
        &matcher,
        &said(1.0, "I start"),
        decoding(1.95),
        &Reported::default(),
    );
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_not_report_the_second_word_of_a_doubled_command_again() {
    // Given "enter" reported at 1.0–1.25 s and "enter" said again right after it
    let words = said(1.0, "enter enter");
    let first = Finding {
        commands: vec!["press enter".into()],
        start: 1.0,
        end: 1.25,
        placement: Placement::Tail,
    };
    let reported = Reported { until: first.end, commands: first.commands.clone() };
    // When
    let second = decide(&standby(), &words, decoding(1.8), &reported).expect("the second word");
    // Then the session drops it as the same command
    assert!(doubles(Some(&first), &second));
}

#[test]
fn should_doubt_an_english_stop_where_the_russian_recognizer_heard_another_word() {
    // Given the English "stop" of "чтобы зайти" and the Russian words over its span
    let english = said(1.0, "stop zayti");
    let found = decide(&dictating(), &english, decoding(1.6), &Reported::default())
        .expect("an English stop");
    let other = said(1.0, "чтобы зайти");
    let respelled = said(1.0, "stop");
    // When / Then: another word doubts it while speech goes on; the Russian "stop" does not
    assert!(doubted_by(&other, 0.0, &found, decoding(1.6)));
    assert!(!doubted_by(&respelled, 0.0, &found, decoding(1.6)));
}

#[test]
fn should_take_a_doubted_stop_ending_the_words_at_the_pause_right_after_it() {
    // Given an English "stop" ending at 1.25 s, the Russian recognizer hearing another word or
    // not decoded past it yet
    let found = decide(
        &dictating(),
        &said(1.0, "stop"),
        decoding(1.5),
        &Reported::default(),
    )
    .expect("an English stop");
    let other = said(1.0, "стопка");
    // When / Then
    assert!(doubted_by(&other, 0.0, &found, decoding(1.5)));
    assert!(!doubted_by(&other, 0.0, &found, paused_at(1.5, 1.3)));
    assert!(doubted_by(&[], 1.3, &found, decoding(1.5)));
    assert!(!doubted_by(&[], 1.3, &found, paused_at(1.5, 1.3)));
}

#[test]
fn should_take_an_english_stop_at_once_once_the_russian_recognizer_decoded_past_it_hearing_nothing()
{
    // Given an English "stop" ending at 1.25 s, the Russian recognizer hearing nothing over it
    let found = decide(
        &dictating(),
        &said(1.0, "stop"),
        decoding(1.5),
        &Reported::default(),
    )
    .expect("an English stop");
    // When / Then
    assert!(!doubted_by(&[], found.end + 0.2, &found, decoding(1.5)));
}
