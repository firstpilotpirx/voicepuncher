use super::*;

#[test]
fn should_parse_a_phrase_selection() {
    // When
    let selection: PhraseSelection =
        serde_json::from_str(r#"{"phrases": ["press enter", "clear all text"]}"#).unwrap();
    // Then
    assert_eq!(selection.phrases, ["press enter", "clear all text"]);
}

#[test]
fn should_read_a_selection_with_and_without_early_start() {
    // When
    let told: PhraseSelection =
        serde_json::from_str(r#"{"phrases": ["press enter"], "early_start": false}"#).unwrap();
    let untold: PhraseSelection = serde_json::from_str(r#"{"phrases": ["press enter"]}"#).unwrap();
    // Then
    assert_eq!((told.early_start, untold.early_start), (Some(false), None));
}

#[test]
fn should_parse_a_sync_request() {
    // When
    let message: AppMessage = serde_json::from_str(r#"{"type":"sync","id":7}"#).unwrap();
    // Then
    assert_eq!(message, AppMessage::Sync { id: 7 });
}

#[test]
fn should_parse_a_hello_with_its_version() {
    // When
    let message: AppMessage = serde_json::from_str(r#"{"type":"hello","version":3}"#).unwrap();
    // Then
    assert_eq!(message, AppMessage::Hello { version: PROTOCOL_VERSION });
}

#[test]
fn should_parse_a_tagged_phrase_selection() {
    // When
    let message: AppMessage =
        serde_json::from_str(r#"{"type":"select","phrases":["press enter"]}"#).unwrap();
    // Then
    assert_eq!(
        message,
        AppMessage::Select(PhraseSelection {
            phrases: vec!["press enter".into()],
            ..PhraseSelection::default()
        })
    );
}

#[test]
fn should_refuse_a_frame_of_partial_samples() {
    // When
    let refused = samples_from_bytes(&[0u8; 5]).unwrap_err();
    // Then
    assert!(refused.contains("5 bytes"), "{refused}");
}

#[test]
fn should_write_the_answer_to_a_sync() {
    // When
    let json = serde_json::to_string(&ServerEvent::Synced { id: 7 }).unwrap();
    // Then
    assert_eq!(json, r#"{"event":"synced","id":7}"#);
}

#[test]
fn should_write_events_with_their_kind() {
    // Given
    let phrase =
        ServerEvent::Phrase { phrase: "press enter".into(), start: 1.5, end: 2.25, timing: None };
    // When
    let json = serde_json::to_string(&phrase).unwrap();
    // Then
    assert_eq!(
        json,
        r#"{"event":"phrase","phrase":"press enter","start":1.5,"end":2.25}"#
    );
    assert_eq!(
        serde_json::to_string(&ServerEvent::SpeechEnd { t: 3.0 }).unwrap(),
        r#"{"event":"speech_end","t":3.0}"#
    );
}

#[test]
fn should_write_a_command_with_its_placement() {
    // Given
    let command = ServerEvent::Command {
        commands: vec!["stop dictation".into(), "press enter".into()],
        start: 4.5,
        end: 5.25,
        placement: Placement::Anywhere,
        timing: None,
    };
    // When
    let json = serde_json::to_string(&command).unwrap();
    // Then
    assert_eq!(
        json,
        r#"{"event":"command","commands":["stop dictation","press enter"],"start":4.5,"end":5.25,"placement":"anywhere"}"#
    );
}

#[test]
fn should_write_a_confirmed_command_with_its_timing() {
    // Given
    let command = ServerEvent::Command {
        commands: vec!["start dictation".into()],
        start: 4.5,
        end: 5.25,
        placement: Placement::Anywhere,
        timing: Some(Timing { asked_at: 5.5, took_ms: 210 }),
    };
    // When
    let json = serde_json::to_string(&command).unwrap();
    // Then
    assert_eq!(
        json,
        r#"{"event":"command","commands":["start dictation"],"start":4.5,"end":5.25,"placement":"anywhere","timing":{"asked_at":5.5,"took_ms":210}}"#
    );
}

#[test]
fn should_write_a_confirmed_phrase_with_its_timing() {
    // Given
    let phrase = ServerEvent::Phrase {
        phrase: "start dictation".into(),
        start: 1.5,
        end: 2.25,
        timing: Some(Timing { asked_at: 2.75, took_ms: 180 }),
    };
    // When
    let json = serde_json::to_string(&phrase).unwrap();
    // Then
    assert_eq!(
        json,
        r#"{"event":"phrase","phrase":"start dictation","start":1.5,"end":2.25,"timing":{"asked_at":2.75,"took_ms":180}}"#
    );
}

#[test]
fn should_write_a_failed_confirmation_a_stall_and_dropped_audio() {
    let json = |event: &ServerEvent| serde_json::to_string(event).unwrap();
    assert_eq!(
        json(&ServerEvent::ConfirmationFailed { start: 1.5, end: 2.25 }),
        r#"{"event":"confirmation_failed","start":1.5,"end":2.25}"#
    );
    assert_eq!(
        json(&ServerEvent::Stalled { waited_ms: 12_000 }),
        r#"{"event":"stalled","waited_ms":12000}"#
    );
    assert_eq!(
        json(&ServerEvent::Dropped { samples: 1_600 }),
        r#"{"event":"dropped","samples":1600}"#
    );
}

#[test]
fn should_read_little_endian_float_samples() {
    // Given
    let bytes: Vec<u8> = [0.5f32, -1.0]
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    // When
    let samples = samples_from_bytes(&bytes).unwrap();
    // Then
    assert_eq!(samples, [0.5, -1.0]);
}

#[test]
fn should_read_a_selection_that_turns_confirmation_off() {
    // When
    let off: PhraseSelection =
        serde_json::from_str(r#"{"phrases": ["press enter"], "confirm": false}"#).unwrap();
    let untold: PhraseSelection = serde_json::from_str(r#"{"phrases": ["press enter"]}"#).unwrap();
    // Then
    assert_eq!((off.confirms(), untold.confirms()), (false, true));
}

#[test]
fn should_write_what_whisper_heard_of_an_unconfirmed_command() {
    // Given
    let heard = ServerEvent::Heard {
        commands: vec!["start dictation".into()],
        start: 1.5,
        end: 2.25,
        verdict: Verdict::Agreed,
    };
    // When
    let json = serde_json::to_string(&heard).unwrap();
    // Then
    assert_eq!(
        json,
        r#"{"event":"heard","commands":["start dictation"],"start":1.5,"end":2.25,"verdict":"agreed"}"#
    );
}
