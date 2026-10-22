use super::*;
use crate::core::domain::{CommandPlacement, ConfirmationTiming, ReportTiming};

#[test]
fn should_read_service_times_as_instants_from_the_audio_start() {
    // Given
    let json = r#"{"event":"phrase","phrase":"press enter","start":1.5,"end":2.25}"#;
    let event: ServiceEvent = serde_json::from_str(json).unwrap();
    let audio_start = Instant::now();
    // When
    let converted = to_spotting_event(event, audio_start, audio_start);
    // Then
    assert_eq!(
        converted.unwrap(),
        SpottingEvent::PhraseSpotted {
            phrase: "press enter".into(),
            start: audio_start + Duration::from_millis(1500),
            end: audio_start + Duration::from_millis(2250),
            timing: Some(ReportTiming {
                words_ended: audio_start + Duration::from_millis(2250),
                confirmation: None,
                arrived_at: audio_start,
            }),
        }
    );
}

#[test]
fn should_read_commands_said_without_a_pause() {
    // Given
    let json = r#"{"event":"command","commands":["stop dictation","press enter"],"start":4.0,"end":4.75,"placement":"anywhere"}"#;
    let event: ServiceEvent = serde_json::from_str(json).unwrap();
    let audio_start = Instant::now();
    // When
    let converted = to_spotting_event(event, audio_start, audio_start);
    // Then
    assert_eq!(
        converted.unwrap(),
        SpottingEvent::CommandsSaid {
            commands: vec![VoiceCommand::StopDictation, VoiceCommand::PressEnter],
            start: audio_start + Duration::from_secs(4),
            end: audio_start + Duration::from_millis(4750),
            placement: CommandPlacement::Anywhere,
            timing: Some(ReportTiming {
                words_ended: audio_start + Duration::from_millis(4750),
                confirmation: None,
                arrived_at: audio_start,
            }),
        }
    );
}

#[test]
fn should_read_the_timing_of_a_confirmed_command() {
    // Given
    let json = r#"{"event":"command","commands":["start dictation"],"start":4.0,"end":4.75,"placement":"head","timing":{"asked_at":5.0,"took_ms":250}}"#;
    let event: ServiceEvent = serde_json::from_str(json).unwrap();
    let audio_start = Instant::now();
    let arrived_at = audio_start + Duration::from_secs(6);
    // When
    let converted = to_spotting_event(event, audio_start, arrived_at);
    // Then
    assert_eq!(
        converted.unwrap(),
        SpottingEvent::CommandsSaid {
            commands: vec![VoiceCommand::StartDictation],
            start: audio_start + Duration::from_secs(4),
            end: audio_start + Duration::from_millis(4750),
            placement: CommandPlacement::Head,
            timing: Some(ReportTiming {
                words_ended: audio_start + Duration::from_millis(4750),
                confirmation: Some(ConfirmationTiming {
                    asked_at: audio_start + Duration::from_secs(5),
                    took: Duration::from_millis(250),
                }),
                arrived_at,
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
    assert!(matches!(start, ServiceEvent::SpeechStart { t } if t == 3.0));
    assert!(matches!(end, ServiceEvent::SpeechEnd { t } if t == 4.5));
}

#[test]
fn should_read_the_answer_to_a_sync() {
    let synced: ServiceEvent = serde_json::from_str(r#"{"event":"synced","id":7}"#).unwrap();
    assert!(matches!(synced, ServiceEvent::Synced { id: 7 }));
}

#[test]
fn should_read_a_failed_confirmation_as_its_segment() {
    // Given
    let json = r#"{"event":"confirmation_failed","start":1.5,"end":2.25}"#;
    let event: ServiceEvent = serde_json::from_str(json).unwrap();
    let audio_start = Instant::now();
    // When
    let converted = to_spotting_event(event, audio_start, audio_start);
    // Then
    assert_eq!(
        converted.unwrap(),
        SpottingEvent::ConfirmationFailed {
            start: audio_start + Duration::from_millis(1500),
            end: audio_start + Duration::from_millis(2250),
        }
    );
}

#[test]
fn should_read_a_stall_and_dropped_audio_as_no_spotting_event() {
    for json in [
        r#"{"event":"stalled","waited_ms":12000}"#,
        r#"{"event":"dropped","samples":1600}"#,
    ] {
        let event: ServiceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(
            to_spotting_event(event, Instant::now(), Instant::now()),
            None,
            "{json}"
        );
    }
}
