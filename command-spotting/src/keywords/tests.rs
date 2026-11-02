use super::*;

fn phrases(list: &[&str]) -> Vec<String> {
    list.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn should_parse_the_one_word_phrases_of_voice_control() {
    // When
    let keywords = parse(KEYWORDS);
    // Then
    let mut names: Vec<&str> = keywords
        .iter()
        .map(|k| k.phrase.as_str())
        .filter(|phrase| !is_trigger(phrase))
        .collect();
    names.dedup();
    assert_eq!(names, ["press enter", "clear all text", "paste text"]);
}

#[test]
fn should_select_only_the_requested_phrases() {
    // Given
    let keywords = parse(KEYWORDS);
    // When
    let selected = select(&keywords, &phrases(&["press enter", "stop dictation"])).unwrap();
    // Then
    assert!(selected.contains("@press_enter"));
    assert!(!selected.contains("@clear_all_text"));
    assert!(!selected.contains("_trigger"));
}

#[test]
fn should_select_the_triggers_of_the_selected_phrases_only() {
    // Given
    let keywords = parse(KEYWORDS);
    // When
    let standby = select_triggers(&keywords, &phrases(&["start dictation", "press enter"]));
    let dictating = select_triggers(&keywords, &phrases(&["restart dictation"]));
    let none = select_triggers(&keywords, &phrases(&["press enter"]));
    // Then
    let standby = standby.expect("start has a trigger");
    assert!(standby.contains("@start_trigger") && !standby.contains("@restart_trigger"));
    assert!(!standby.contains("@start_dictation"));
    assert!(dictating.is_some_and(|d| d.contains("@restart_trigger")));
    assert_eq!(none, None);
}

#[test]
fn should_never_select_a_trigger_as_a_phrase() {
    // Given
    let keywords = parse(KEYWORDS);
    // When
    let selected = select(&keywords, &phrases(&["start dictation"])).unwrap();
    // Then
    assert_eq!(selected, "");
    assert!(is_trigger("start trigger") && !is_trigger("start dictation"));
}

#[test]
fn should_name_a_spotted_keyword_by_its_phrase() {
    assert_eq!(phrase_of("clear_all_text"), "clear all text");
}

#[test]
fn should_reject_an_unknown_phrase_by_name() {
    // Given
    let keywords = parse(KEYWORDS);
    // When
    let error = select(&keywords, &phrases(&["press enter", "open the door"])).unwrap_err();
    // Then
    assert_eq!(error, UnknownPhrases(phrases(&["open the door"])));
    assert_eq!(error.to_string(), "unknown phrases: open the door");
}

#[test]
fn should_spot_every_declared_command_by_a_line_or_a_trigger_and_nothing_else() {
    // When
    let keywords = parse(KEYWORDS);
    // Then
    let spotted: Vec<&str> = keywords
        .iter()
        .map(|k| k.phrase.as_str())
        .filter(|phrase| !is_trigger(phrase))
        .collect();
    for phrase in &spotted {
        assert!(
            voicepuncher_voice_commands::COMMANDS.contains(phrase),
            "keywords.txt spots {phrase:?}, which voice-commands does not declare"
        );
    }
    for phrase in voicepuncher_voice_commands::COMMANDS {
        let triggered = keywords
            .iter()
            .any(|k| served_by(&k.phrase) == Some(phrase));
        assert!(
            spotted.contains(&phrase) || triggered,
            "keywords.txt has no line or trigger for {phrase:?}"
        );
    }
}
