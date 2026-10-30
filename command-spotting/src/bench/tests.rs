use super::*;

fn outcome(label: &[&str], heard: &[&str], delay_ms: Option<f64>) -> Outcome {
    Outcome {
        id: "clip".to_owned(),
        label: label.iter().map(|w| (*w).to_owned()).collect(),
        heard: heard.iter().map(|w| (*w).to_owned()).collect(),
        delay_ms,
    }
}

#[test]
fn should_count_a_reported_phrase_as_its_label_word() {
    assert_eq!(label_word("press enter"), "enter");
    assert_eq!(label_word("stop dictation"), "stop");
    assert_eq!(label_word("clear all text"), "clear");
}

#[test]
fn should_count_a_doubled_command_heard_once_as_right() {
    // Given
    let doubled = outcome(&["start", "start"], &["start"], None);
    // Then
    assert!(doubled.right());
}

#[test]
fn should_count_a_missed_or_wrong_command_as_wrong() {
    assert!(!outcome(&["stop"], &[], None).right());
    assert!(!outcome(&["stop", "enter"], &["enter", "stop"], None).right());
}

#[test]
fn should_score_commands_false_alarms_and_delays() {
    // Given
    let outcomes = [
        outcome(&["start"], &["start"], Some(300.0)),
        outcome(&["stop"], &["stop"], Some(100.0)),
        outcome(&["stop"], &[], None),
        outcome(&["enter"], &["enter"], Some(200.0)),
        outcome(&[], &["enter"], None),
        outcome(&[], &[], None),
    ];
    // When
    let summary = Summary::of(&outcomes);
    // Then
    assert_eq!(
        summary,
        Summary {
            commands: 4,
            right: 3,
            negatives: 2,
            false_alarms: 1,
            delays: 3,
            median_ms: Some(200.0),
            p90_ms: Some(300.0),
        }
    );
}

#[test]
fn should_print_the_scores_on_one_line() {
    // Given
    let summary = Summary::of(&[outcome(&["start"], &["start"], Some(250.0))]);
    // Then
    assert_eq!(
        summary.to_string(),
        "right 1/1 (100.0%) · false alarms 0/0 · delay median 250ms p90 250ms of 1"
    );
}
