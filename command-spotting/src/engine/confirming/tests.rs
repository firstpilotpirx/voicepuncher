use super::*;
use crate::matcher::Placement;

fn standby() -> PhraseMatcher {
    PhraseMatcher::new(&[
        "start dictation".into(),
        "press enter".into(),
        "clear all text".into(),
    ])
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

#[test]
fn should_take_a_doubled_start_at_the_head_of_a_running_utterance() {
    // Given the start of an utterance heard as "start start" and the dictation after it
    let said = words("Start, start, we meet on Friday");
    // When
    let found = head_command(&standby(), &said);
    // Then
    let found = found.expect("start start is a command");
    assert_eq!(found.commands, ["start dictation"]);
    assert_eq!((found.first, found.last), (0, 1));
}

#[test]
fn should_take_a_chain_ending_in_start_at_the_head_of_a_running_utterance() {
    // Given the start of an utterance heard as "enter start" and the dictation after it
    let said = words("Enter, start, we meet on Friday");
    // When
    let found = head_command(&standby(), &said);
    // Then
    let found = found.expect("enter start is a chain");
    assert_eq!(found.commands, ["press enter", "start dictation"]);
    assert_eq!(found.placement, Placement::Anywhere);
}

#[test]
fn should_leave_doubled_commands_other_than_start_to_the_later_checks() {
    // Given a short utterance heard as a doubled "clear": its segment reports it once
    let said = words("Fine, clear clear");
    // When
    let at_head = head_command(&standby(), &said);
    let spotted = spotted_command(&standby(), &said, false, Round::First);
    // Then
    assert_eq!((at_head, spotted), (None, None));
}

#[test]
fn should_not_take_start_heard_alone_by_the_first_spotted_check() {
    // Given the first check of a spotted "start" heard nothing after it yet
    let said = words("Start.");
    // When
    let found = spotted_command(&standby(), &said, true, Round::First);
    // Then
    assert_eq!(found, None);
}

#[test]
fn should_take_start_please_heard_alone_by_the_first_spotted_check() {
    // Given the first check of a spotted "start" heard a start closed by "please" and
    // nothing after it
    for said in ["Start please.", "Start dictation, please."] {
        // When
        let found = spotted_command(&standby(), &words(said), true, Round::First);
        // Then
        assert_eq!(
            found.map(|f| f.commands),
            Some(vec!["start dictation".to_owned()]),
            "{said}"
        );
    }
}

#[test]
fn should_take_start_heard_alone_by_the_second_spotted_check() {
    // Given the second check of a spotted "start" still heard "start" alone
    let said = words("Start.");
    // When
    let found = spotted_command(&standby(), &said, true, Round::Second);
    // Then
    assert_eq!(
        found.map(|f| f.commands),
        Some(vec!["start dictation".to_owned()])
    );
}

#[test]
fn should_take_start_opening_speech_by_the_first_spotted_check() {
    // Given
    let said = words("Start, we meet on Friday");
    // When
    let found = spotted_command(&standby(), &said, true, Round::First);
    // Then
    assert_eq!(found.map(|f| f.placement), Some(Placement::Head));
}

#[test]
fn should_take_any_doubled_command_heard_by_a_doubled_check() {
    // Given a doubled "enter" heard inside speech
    let said = words("Fine, enter enter, and then");
    // When
    let found = spotted_command(&standby(), &said, false, Round::Doubled);
    // Then
    assert_eq!(
        found.map(|f| f.commands),
        Some(vec!["press enter".to_owned()])
    );
}

#[test]
fn should_leave_a_chain_to_the_utterance_checks_in_a_check_after_a_short_cancel() {
    // Given "cancel clear" heard by the check a short "cancel" asked
    let matcher = PhraseMatcher::new(&["cancel dictation".into(), "clear all text".into()]);
    let said = words("cancel clear");
    // When
    let found = spotted_command(&matcher, &said, false, Round::Short);
    // Then
    assert_eq!(found.map(|f| f.commands), None);
}

#[test]
fn should_take_a_doubled_cancel_in_a_check_after_a_short_cancel() {
    // Given "cancel cancel" heard by the check a short "cancel" asked
    let matcher = PhraseMatcher::new(&["cancel dictation".into(), "clear all text".into()]);
    let said = words("we meet at ten, cancel cancel");
    // When
    let found = spotted_command(&matcher, &said, false, Round::Short);
    // Then
    assert_eq!(
        found.map(|f| f.commands),
        Some(vec!["cancel dictation".to_owned()])
    );
}
