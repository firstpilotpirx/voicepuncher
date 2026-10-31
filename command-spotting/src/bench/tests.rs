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

fn row(json: &str) -> Row {
    serde_json::from_str(json).expect("a row")
}

#[test]
fn should_cut_each_recorded_row_between_its_neighbours() {
    // Given three rows of a session, the second said in standby, the last unclear
    let rows = [
        row(r#"{"start": 1.0, "end": 1.5, "state": "dictating", "commands": ["stop dictation"]}"#),
        row(
            r#"{"start": 1.8, "end": 2.2, "state": "standby", "commands": ["start dictation", "press enter"]}"#,
        ),
        row(r#"{"start": 2.9, "end": 3.0, "state": "standby", "commands": [], "unclear": true}"#),
    ];
    // When
    let spans = spans(&rows);
    // Then
    assert_eq!(spans.len(), 2);
    let (n, stop) = &spans[0];
    assert_eq!(*n, 0);
    assert!(
        (stop.from - 0.4).abs() < 1e-4 && (stop.to - 1.8).abs() < 1e-4,
        "{stop:?}"
    );
    assert_eq!(stop.label, ["stop"]);
    assert!((stop.word_end.expect("one command") - 1.1).abs() < 1e-4);
    assert!(stop.listening.contains(&"stop dictation".to_owned()));
    let (_, chain) = &spans[1];
    assert!(
        (chain.from - 1.5).abs() < 1e-4 && (chain.to - 2.9).abs() < 1e-4,
        "{chain:?}"
    );
    assert_eq!(chain.label, ["start", "enter"]);
    assert_eq!(chain.word_end, None);
    assert!(chain.listening.contains(&"start dictation".to_owned()));
}

#[test]
fn should_read_session_ones_manifest_row() {
    // Given
    let row = row(
        r#"{"file": "01-start-dictation.wav", "start": 4.98, "end": 6.19, "command": "start dictation", "unclear": false}"#,
    );
    // When
    let spans = spans(&[row]);
    // Then
    assert_eq!(spans[0].1.label, ["start"]);
    assert!(spans[0].1.listening.contains(&"start dictation".to_owned()));
}
