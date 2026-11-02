use super::*;
use crate::core::domain::{CommandPlacement, ReportTiming};

#[test]
fn should_read_commands_said_without_a_pause() {
    // Given
    let json = r#"{"event":"command","commands":["stop dictation","press enter"],"start":4.0,"end":4.75,"placement":"anywhere"}"#;
    let audio_start = Instant::now();
    // When
    let converted = arrived_at_the_start(json, audio_start);
    // Then
    assert_eq!(
        converted,
        SpottingEvent::CommandsSaid {
            commands: vec![VoiceCommand::StopDictation, VoiceCommand::PressEnter],
            start: audio_start + Duration::from_secs(4),
            end: audio_start + Duration::from_millis(4750),
            placement: CommandPlacement::Anywhere,
            timing: Some(ReportTiming {
                words_ended: audio_start + Duration::from_millis(4750),
                arrived_at: audio_start,
            }),
        }
    );
}

#[test]
fn should_place_a_negative_service_time_at_the_audio_start() {
    // Given
    let event: ServiceEvent = serde_json::from_str(r#"{"event":"speech_start","t":-0.1}"#).unwrap();
    let audio_start = Instant::now();
    // When
    let converted = to_spotting_event(event, audio_start, audio_start);
    // Then
    assert_eq!(converted, Some(SpottingEvent::SpeechStarted(audio_start)));
}

#[test]
fn should_read_speech_bounds() {
    let start: ServiceEvent = serde_json::from_str(r#"{"event":"speech_start","t":3.0}"#).unwrap();
    let end: ServiceEvent = serde_json::from_str(r#"{"event":"speech_end","t":4.5}"#).unwrap();
    assert!(matches!(start, ServiceEvent::SpeechStart { t } if (t - 3.0).abs() < 1e-6));
    assert!(matches!(end, ServiceEvent::SpeechEnd { t } if (t - 4.5).abs() < 1e-6));
}

#[test]
fn should_read_the_answer_to_a_sync() {
    let synced: ServiceEvent = serde_json::from_str(r#"{"event":"synced","id":7}"#).unwrap();
    assert!(matches!(synced, ServiceEvent::Synced { id: 7 }));
}

#[test]
fn should_read_dropped_audio_as_no_spotting_event() {
    // Given
    let json = r#"{"event":"dropped","samples":1600}"#;
    let event: ServiceEvent = serde_json::from_str(json).unwrap();
    // When
    let converted = to_spotting_event(event, Instant::now(), Instant::now());
    // Then
    assert_eq!(converted, None);
}

#[test]
fn should_ask_for_an_early_start_only_while_start_dictation_is_listened_for() {
    // When
    let early = |commands| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&selection_of(commands)).unwrap()["early_start"]
            .clone()
    };
    // Then
    assert_eq!(early(&VoiceCommand::STANDBY), serde_json::json!(true));
    assert_eq!(early(&VoiceCommand::DICTATING), serde_json::json!(false));
}

/// The event the service line `json` becomes when it arrives at the audio start.
fn arrived_at_the_start(json: &str, audio_start: Instant) -> SpottingEvent {
    let event: ServiceEvent = serde_json::from_str(json).unwrap();
    to_spotting_event(event, audio_start, audio_start).unwrap()
}
