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
    let phrase = combined.phrase_in("Start dictation.");
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
    assert_eq!(
        standby().phrase_in("Press Enter."),
        Some("press enter".into())
    );
    assert_eq!(
        dictating().phrase_in(" stop dictation "),
        Some("stop dictation".into())
    );
}

#[test]
fn should_take_a_phrase_followed_by_please() {
    assert_eq!(
        standby().phrase_in("Press enter, please."),
        Some("press enter".into())
    );
    assert_eq!(
        dictating().phrase_in("Cancel dictation please"),
        Some("cancel dictation".into())
    );
}

#[test]
fn should_take_clear_text_as_clear_all_text() {
    assert_eq!(
        standby().phrase_in("Clear text."),
        Some("clear all text".into())
    );
    assert_eq!(
        standby().phrase_in("Clear text, please."),
        Some("clear all text".into())
    );
}

#[test]
fn should_take_the_slips_heard_in_the_users_voice() {
    assert_eq!(
        standby().phrase_in("Star dictation."),
        Some("start dictation".into())
    );
    assert_eq!(
        standby().phrase_in("Start.\n Activation."),
        Some("start dictation".into())
    );
    assert_eq!(
        standby().phrase_in("Presenter."),
        Some("press enter".into())
    );
    assert_eq!(
        standby().phrase_in("Clear all texts."),
        Some("clear all text".into())
    );
    assert_eq!(
        dictating().phrase_in("Cancel the quotation please."),
        Some("cancel dictation".into())
    );
}

#[test]
fn should_take_capsule_dictation_for_cancel_dictation() {
    assert_eq!(
        dictating().phrase_in("Capsule dictation."),
        Some("cancel dictation".into())
    );
}

#[test]
fn should_take_a_phrase_said_twice_once() {
    assert_eq!(
        standby().phrase_in("Star dictation, star dictation."),
        Some("start dictation".into())
    );
    assert_eq!(
        standby().phrase_in("Press enter. Press enter."),
        Some("press enter".into())
    );
}

#[test]
fn should_take_a_repeat_whose_sentences_say_the_same_command() {
    assert_eq!(
        dictating().phrase_in(" Cancel dictation please. Cancel dictation.\n"),
        Some("cancel dictation".into())
    );
}

#[test]
fn should_not_take_sentences_that_say_different_things() {
    assert_eq!(standby().phrase_in("Press enter. Hello."), None);
    assert_eq!(standby().phrase_in("Press enter. Clear text."), None);
}

#[test]
fn should_not_take_a_phrase_with_other_words_around_it() {
    for transcript in [
        "Please press enter.",
        "Press enter when you are ready.",
        "I think we should press enter.",
        "Let's start dictation tomorrow.",
    ] {
        assert_eq!(standby().phrase_in(transcript), None, "{transcript}");
    }
}

#[test]
fn should_take_the_short_forms() {
    for (transcript, phrase) in [
        ("Enter.", "press enter"),
        ("Enter, please.", "press enter"),
        ("Clear.", "clear all text"),
        ("Clear please.", "clear all text"),
        ("Paste text.", "paste text"),
        ("Paste.", "paste text"),
        ("Paste please.", "paste text"),
        ("Start.", "start dictation"),
        ("Start please.", "start dictation"),
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
        dictating().phrase_in("Stop, please."),
        Some("stop dictation".into())
    );
    assert_eq!(
        dictating().phrase_in("Cancel."),
        Some("cancel dictation".into())
    );
    assert_eq!(
        dictating().phrase_in("Restart, please."),
        Some("restart dictation".into())
    );
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
    assert_eq!(standby().phrase_in("Stop dictation."), None);
    assert_eq!(dictating().phrase_in("Clear all text."), None);
}

#[test]
fn should_read_every_alias_row_as_its_command() {
    for row in MISHEARINGS
        .iter()
        .filter(|m| m.kind == MishearingKind::Alias)
    {
        // Given a matcher selecting the command the row stands for
        let matcher = PhraseMatcher::new(&[row.means[0].to_owned()]);
        // When
        let phrase = matcher.phrase_in(&row.heard.join(" "));
        // Then
        assert_eq!(phrase.as_deref(), Some(row.means[0]), "{:?}", row.heard);
    }
}

#[test]
fn should_read_an_alias_row_ending_speech_as_its_command() {
    for row in MISHEARINGS
        .iter()
        .filter(|m| m.kind == MishearingKind::Alias)
    {
        // Given the row's words said at the end of a sentence, its command selected
        let said = format!("we meet on friday {}", row.heard.join(" "));
        let words: Vec<&str> = said.split(' ').collect();
        let matcher = PhraseMatcher::new(&[row.means[0].to_owned()]);
        // When
        let found = matcher.at_end(&words);
        // Then
        let expected = vec![row.means[0].to_owned()];
        assert_eq!(found.map(|f| f.commands), Some(expected), "{said}");
    }
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
    assert_eq!(words("Стоп, пожалуйста."), ["стоп", "пожалуйста"]);
}
