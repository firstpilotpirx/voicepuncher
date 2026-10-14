//! The app's side of the command spotting service protocol: phrase selection and audio
//! out, spotting events in. Service times are seconds since the first sample sent; the
//! client turns them into instants from the moment that sample was captured.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::core::domain::{CommandPlacement, SpottingEvent, VoiceCommand};

/// How long a read waits for an event before the caller gets control back.
const READ_WAIT: Duration = Duration::from_millis(10);
/// How long a send may block on a service that stopped reading.
const WRITE_WAIT: Duration = Duration::from_secs(2);
/// How often the client checks that the service still answers.
const PING_EVERY: Duration = Duration::from_secs(2);
/// How long the service may stay silent, pings included, before it counts as gone.
const SILENCE_LIMIT: Duration = Duration::from_secs(10);
const SAMPLE_BYTES: usize = 4;

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ServiceEvent {
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
    Command {
        commands: Vec<String>,
        start: f32,
        end: f32,
        placement: Placement,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Placement {
    Tail,
    Head,
    Anywhere,
}

pub struct SpottingClient {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    audio_start: Instant,
    last_answer: Instant,
    last_ping: Instant,
}

impl SpottingClient {
    /// Connects to the service; `audio_start` is when the first sample to be sent was captured.
    pub fn connect(url: &str, audio_start: Instant) -> Result<Self> {
        let (socket, _) = tungstenite::connect(url).with_context(|| {
            format!(
                "command spotting service not reachable at {url} — run 'just start-model-servers'"
            )
        })?;
        if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
            stream.set_read_timeout(Some(READ_WAIT))?;
            stream.set_write_timeout(Some(WRITE_WAIT))?;
        }
        let now = Instant::now();
        Ok(Self {
            socket,
            audio_start,
            last_answer: now,
            last_ping: now,
        })
    }

    /// Spots the phrases of `commands` from now on.
    pub fn listen_for(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        let phrases: Vec<&str> = commands.iter().map(|c| c.phrase()).collect();
        let selection = serde_json::json!({ "phrases": phrases }).to_string();
        self.socket.send(Message::text(selection))?;
        Ok(())
    }

    /// Sends 16 kHz mono samples.
    pub fn send_audio(&mut self, samples: &[f32]) -> Result<()> {
        let mut bytes = Vec::with_capacity(samples.len() * SAMPLE_BYTES);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.socket.send(Message::binary(bytes))?;
        Ok(())
    }

    /// Pings the service now and then; fails when it has not answered anything, pings
    /// included, for longer than the silence limit — a service that hangs.
    pub fn keep_alive(&mut self) -> Result<()> {
        let silent = self.last_answer.elapsed();
        if silent > SILENCE_LIMIT {
            bail!(
                "command spotting service stopped answering ({}s without a reply)",
                silent.as_secs()
            );
        }
        if self.last_ping.elapsed() >= PING_EVERY {
            self.socket.send(Message::Ping(Default::default()))?;
            self.last_ping = Instant::now();
        }
        Ok(())
    }

    /// Every event that arrived, waiting briefly for the first; an error means the
    /// service is gone.
    pub fn read_events(&mut self) -> Result<Vec<SpottingEvent>> {
        let mut events = Vec::new();
        loop {
            let message = self.socket.read();
            if message.is_ok() {
                self.last_answer = Instant::now();
            }
            match message {
                Ok(Message::Text(text)) => events.extend(self.parse(text.as_str())),
                Ok(Message::Close(frame)) => bail!("the service closed the connection: {frame:?}"),
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if is_timeout(&e) => return Ok(events),
                Err(e) => {
                    return Err(anyhow!(e).context("command spotting service connection lost"))
                }
            }
        }
    }

    pub fn close(mut self) {
        let _ = self.socket.close(None);
        let _ = self.socket.flush();
    }

    fn parse(&self, text: &str) -> Option<SpottingEvent> {
        match serde_json::from_str::<ServiceEvent>(text) {
            Ok(event) => Some(to_spotting_event(event, self.audio_start)),
            Err(e) => {
                log::warn!("command spotting service sent an unknown event {text}: {e}");
                None
            }
        }
    }
}

/// A service event with its times, seconds since the first sample, as instants from
/// `audio_start`, the moment that sample was captured.
fn to_spotting_event(event: ServiceEvent, audio_start: Instant) -> SpottingEvent {
    let at = |secs: f32| audio_start + Duration::from_secs_f32(secs.max(0.0));
    match event {
        ServiceEvent::SpeechStart { t } => SpottingEvent::SpeechStarted(at(t)),
        ServiceEvent::SpeechEnd { t } => SpottingEvent::SpeechEnded(at(t)),
        ServiceEvent::Phrase { phrase, start, end } => SpottingEvent::PhraseSpotted {
            phrase,
            start: at(start),
            end: at(end),
        },
        ServiceEvent::Command {
            commands,
            start,
            end,
            placement,
        } => SpottingEvent::CommandsSaid {
            commands: commands
                .iter()
                .filter_map(|phrase| VoiceCommand::from_phrase(phrase))
                .collect(),
            start: at(start),
            end: at(end),
            placement: match placement {
                Placement::Tail => CommandPlacement::Tail,
                Placement::Head => CommandPlacement::Head,
                Placement::Anywhere => CommandPlacement::Anywhere,
            },
        },
    }
}

/// Fails when nothing answers at `url`.
pub fn probe(url: &str) -> Result<()> {
    SpottingClient::connect(url, Instant::now())?.close();
    Ok(())
}

fn is_timeout(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_read_service_times_as_instants_from_the_audio_start() {
        // Given
        let json = r#"{"event":"phrase","phrase":"press enter","start":1.5,"end":2.25}"#;
        let event: ServiceEvent = serde_json::from_str(json).unwrap();
        let audio_start = Instant::now();
        // When
        let converted = to_spotting_event(event, audio_start);
        // Then
        assert_eq!(
            converted,
            SpottingEvent::PhraseSpotted {
                phrase: "press enter".into(),
                start: audio_start + Duration::from_millis(1500),
                end: audio_start + Duration::from_millis(2250),
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
        let converted = to_spotting_event(event, audio_start);
        // Then
        assert_eq!(
            converted,
            SpottingEvent::CommandsSaid {
                commands: vec![VoiceCommand::StopDictation, VoiceCommand::PressEnter],
                start: audio_start + Duration::from_secs(4),
                end: audio_start + Duration::from_millis(4750),
                placement: CommandPlacement::Anywhere,
            }
        );
    }

    #[test]
    fn should_place_a_negative_service_time_at_the_audio_start() {
        // Given
        let event: ServiceEvent =
            serde_json::from_str(r#"{"event":"speech_start","t":-0.1}"#).unwrap();
        let audio_start = Instant::now();
        // When
        let converted = to_spotting_event(event, audio_start);
        // Then
        assert_eq!(converted, SpottingEvent::SpeechStarted(audio_start));
    }

    #[test]
    fn should_read_speech_bounds() {
        let start: ServiceEvent =
            serde_json::from_str(r#"{"event":"speech_start","t":3.0}"#).unwrap();
        let end: ServiceEvent = serde_json::from_str(r#"{"event":"speech_end","t":4.5}"#).unwrap();
        assert!(matches!(start, ServiceEvent::SpeechStart { t } if t == 3.0));
        assert!(matches!(end, ServiceEvent::SpeechEnd { t } if t == 4.5));
    }
}
