use super::*;
use voicepuncher_voice_commands::{MishearingKind, MISHEARINGS, STOP};

/// The phrases heard in standby.
pub(super) fn standby() -> PhraseMatcher {
    PhraseMatcher::new(&[
        "start dictation".into(),
        "press enter".into(),
        "clear all text".into(),
        "paste text".into(),
    ])
}

/// The phrases heard during a dictation.
pub(super) fn dictating() -> PhraseMatcher {
    PhraseMatcher::new(&[
        "stop dictation".into(),
        "cancel dictation".into(),
        "restart dictation".into(),
        "press enter".into(),
    ])
}

#[test]
fn should_take_a_start_said_in_standby_as_a_start_once_combined_with_the_dictating_phrases() {
    // Given the standby phrases combined with those of a running dictation
    let combined = standby().combined_with(&dictating());
    // When
    let phrase = combined.phrase_in("Start start.");
    // Then
    assert_eq!(phrase, Some("start dictation".into()));
}

#[test]
fn should_take_a_start_said_while_dictating_as_a_start_once_combined_with_standby() {
    // Given the phrases of a running dictation combined with the standby ones
    let combined = dictating().combined_with(&standby());
    // When
    let phrase = combined.phrase_in("Start.");
    // Then
    assert_eq!(phrase, Some("start dictation".into()));
}

#[test]
fn should_select_the_phrases_of_both_sets_each_once() {
    // Given
    let combined = dictating().combined_with(&standby());
    // When
    let selected = combined.selected();
    // Then
    assert_eq!(
        selected,
        [
            "stop dictation",
            "cancel dictation",
            "restart dictation",
            "press enter",
            "start dictation",
            "clear all text",
            "paste text",
        ]
    );
}

#[test]
fn should_take_an_exact_phrase_whatever_its_case_and_punctuation() {
    assert_eq!(standby().phrase_in("ENTER!"), Some("press enter".into()));
    assert_eq!(
        dictating().phrase_in(" stop "),
        Some("stop dictation".into())
    );
}

#[test]
fn should_take_a_phrase_said_twice_once() {
    assert_eq!(
        standby().phrase_in("Start, start."),
        Some("start dictation".into())
    );
    assert_eq!(
        standby().phrase_in("Enter. Enter."),
        Some("press enter".into())
    );
}

#[test]
fn should_take_a_repeat_whose_sentences_say_the_same_command() {
    assert_eq!(
        dictating().phrase_in(" Cancel. Cancel!\n"),
        Some("cancel dictation".into())
    );
}

#[test]
fn should_not_take_sentences_that_say_different_things() {
    assert_eq!(standby().phrase_in("Enter. Hello."), None);
    assert_eq!(standby().phrase_in("Enter. Clear."), None);
}

#[test]
fn should_not_take_a_phrase_with_other_words_around_it() {
    for transcript in [
        "Please enter.",
        "Enter when you are ready.",
        "I think we should enter.",
        "Let's start tomorrow.",
    ] {
        assert_eq!(standby().phrase_in(transcript), None, "{transcript}");
    }
}

#[test]
fn should_not_take_a_word_close_to_a_one_word_form() {
    // Given a draft of a longer word, as a streaming recognizer decodes "station" of
    // "Start station" in steps
    for (matcher, transcript) in [
        (standby(), "Stat"),
        (standby(), "Enta"),
        (dictating(), "Step"),
    ] {
        assert_eq!(matcher.phrase_in(transcript), None, "{transcript}");
    }
}

#[test]
fn should_take_the_short_forms() {
    for (transcript, phrase) in [
        ("Enter.", "press enter"),
        ("Clear.", "clear all text"),
        ("Paste.", "paste text"),
        ("Start.", "start dictation"),
    ] {
        assert_eq!(
            standby().phrase_in(transcript),
            Some(phrase.into()),
            "{transcript}"
        );
    }
    assert_eq!(
        dictating().phrase_in("Stop."),
        Some("stop dictation".into())
    );
    assert_eq!(
        dictating().phrase_in("Cancel."),
        Some("cancel dictation".into())
    );
    assert_eq!(
        dictating().phrase_in("Restart."),
        Some("restart dictation".into())
    );
}

#[test]
fn should_not_take_a_long_form_of_a_command() {
    for transcript in [
        "Start dictation.",
        "Press enter.",
        "Clear all text.",
        "Clear text.",
        "Paste text.",
        "Presenter.",
        "Star dictation.",
    ] {
        assert_eq!(standby().phrase_in(transcript), None, "{transcript}");
    }
    for transcript in [
        "Stop dictation.",
        "Cancel dictation.",
        "Capsule dictation.",
        "Cancel the quotation.",
        "Restart dictation.",
    ] {
        assert_eq!(dictating().phrase_in(transcript), None, "{transcript}");
    }
}

#[test]
fn should_not_take_other_short_words() {
    for transcript in ["Hello.", "Thank you.", "Okay.", "Yes.", "Restart.", ""] {
        assert_eq!(standby().phrase_in(transcript), None, "{transcript}");
    }
    for transcript in ["Start.", "Let's start.", "Cancelled.", "Stopped.", "Okay."] {
        assert_eq!(dictating().phrase_in(transcript), None, "{transcript}");
    }
}

#[test]
fn should_take_start_alone_or_after_lets_as_a_near_miss_of_restart_while_dictating() {
    for transcript in ["Start.", "Let's start.", "start", "Okay, start."] {
        assert_eq!(
            dictating().near_miss(transcript),
            Some("restart dictation".into()),
            "{transcript}"
        );
    }
}

#[test]
fn should_not_take_a_near_miss_of_a_command_not_selected() {
    assert_eq!(standby().near_miss("Let's start."), None);
}

#[test]
fn should_not_take_start_in_a_longer_sentence_as_a_near_miss() {
    for transcript in [
        "Let's start with the budget.",
        "We start at nine tomorrow.",
        "We are all here, let's start.",
        "The meeting is about to start.",
        "Restart.",
        "Stop.",
        "",
    ] {
        assert_eq!(dictating().near_miss(transcript), None, "{transcript}");
    }
}

#[test]
fn should_not_take_a_phrase_outside_the_selected_set() {
    assert_eq!(standby().phrase_in("Stop."), None);
    assert_eq!(dictating().phrase_in("Clear."), None);
}

#[test]
fn should_close_a_chain_with_an_after_command_row() {
    for row in MISHEARINGS
        .iter()
        .filter(|m| m.kind == MishearingKind::AfterCommand)
    {
        // Given "stop" said before the row's words in a dictation
        let said = format!("stop {}", row.heard.join(" "));
        let words: Vec<&str> = said.split(' ').collect();
        // When
        let found = dictating().anywhere(&words);
        // Then
        let mut expected = vec![STOP.to_owned()];
        expected.extend(row.means.iter().map(|&p| p.to_owned()));
        assert_eq!(found.map(|f| f.commands), Some(expected), "{said}");
    }
}

#[test]
fn should_read_the_words_of_a_cyrillic_transcript() {
    assert_eq!(words("Стоп, коллеги."), ["стоп", "коллеги"]);
}
