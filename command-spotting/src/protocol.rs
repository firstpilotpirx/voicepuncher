//! Messages between the app and the service over the WebSocket connection.

use serde::{Deserialize, Serialize};

pub use crate::matcher::Placement;

/// Text frame from the app: the phrases to spot from now on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PhraseSelection {
    pub phrases: Vec<String>,
}

/// A text frame from the app.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum AppMessage {
    Select(PhraseSelection),
    /// Asks for `synced` with this id once every event for the audio sent before it is out.
    Sync {
        sync: u64,
    },
}

/// Text frame to the app. Times are seconds since the first sample of the connection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ServerEvent {
    SpeechStart {
        t: f32,
    },
    SpeechEnd {
        t: f32,
    },
    Phrase {
        phrase: String,
        start: f32,
        end: f32,
    },
    /// Commands said without a pause around them, to carry out in order without the
    /// app's pause rule: `start..end` spans their words.
    Command {
        commands: Vec<String>,
        start: f32,
        end: f32,
        placement: Placement,
    },
    /// Every event for the audio sent before the sync with this id is out.
    Synced {
        id: u64,
    },
}

const SAMPLE_BYTES: usize = 4;

/// Binary frame from the app: 16 kHz mono `f32` little-endian samples.
pub fn samples_from_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(SAMPLE_BYTES)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
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
    fn should_parse_a_sync_request() {
        // When
        let message: AppMessage = serde_json::from_str(r#"{"sync": 7}"#).unwrap();
        // Then
        assert_eq!(message, AppMessage::Sync { sync: 7 });
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
        let phrase = ServerEvent::Phrase {
            phrase: "press enter".into(),
            start: 1.5,
            end: 2.25,
        };
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
    fn should_read_little_endian_float_samples() {
        // Given
        let bytes: Vec<u8> = [0.5f32, -1.0]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        // When
        let samples = samples_from_bytes(&bytes);
        // Then
        assert_eq!(samples, [0.5, -1.0]);
    }
}
