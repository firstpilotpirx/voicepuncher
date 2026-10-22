//! The app's side of the command spotting service protocol: phrase selection and audio
//! out, spotting events in. Service times are seconds since the first sample sent; the
//! client turns them into instants from the moment that sample was captured.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};
use voicepuncher_copy_pool::{ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET};

use crate::core::domain::{
    CommandPlacement, ConfirmationTiming, ReportTiming, SpottingEvent, VoiceCommand,
};

/// How long a read waits for an event before the caller gets control back.
const READ_WAIT: Duration = Duration::from_millis(10);
/// How long a send may block on a service that stopped reading.
const WRITE_WAIT: Duration = Duration::from_secs(2);
/// How long connecting and the WebSocket handshake may take: a service that hangs fails
/// the attempt instead of holding it.
const CONNECT_WAIT: Duration = Duration::from_secs(2);
/// How often the client checks that the service still answers.
const PING_EVERY: Duration = Duration::from_secs(1);
/// How long a ping may stay without any answer before the service counts as gone: the
/// service answers pings at once, however long its spotting takes.
const SILENCE_LIMIT: Duration = Duration::from_secs(5);
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
        timing: Option<Confirmation>,
    },
    Command {
        commands: Vec<String>,
        start: f32,
        end: f32,
        placement: Placement,
        timing: Option<Confirmation>,
    },
    Synced {
        id: u64,
    },
}

/// The confirmation that heard a phrase or command: the service time it was asked at, and
/// how long it took.
#[derive(Debug, Deserialize)]
struct Confirmation {
    asked_at: f32,
    took_ms: u64,
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
    /// When the oldest ping still without any answer was sent: silence counts from the
    /// moment the client asked, never from a pause of its own.
    unanswered_since: Option<Instant>,
    last_ping: Option<Instant>,
    /// Waits for the service's answer so far, numbering the sync of each.
    answer_waits: u64,
}

impl SpottingClient {
    /// Connects to the service at its own port, else to the first of its copies that
    /// answers; `audio_start` is when the first sample to be sent was captured.
    pub fn connect(url: &str, audio_start: Instant) -> Result<Self> {
        let socket = handshake_any(url).with_context(|| {
            format!(
                "command spotting service not reachable at {url} — run 'just start-model-servers'"
            )
        })?;
        if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
            stream.set_read_timeout(Some(READ_WAIT))?;
            stream.set_write_timeout(Some(WRITE_WAIT))?;
        }
        Ok(Self { socket, audio_start, unanswered_since: None, last_ping: None, answer_waits: 0 })
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

    /// Pings the service at once and then now and then; whether it answers is judged after
    /// the reads, so what it sent during a pause of the client's own counts.
    pub fn keep_alive(&mut self) -> Result<()> {
        if self.last_ping.is_none_or(|at| at.elapsed() >= PING_EVERY) {
            self.socket.send(Message::Ping(Default::default()))?;
            let now = Instant::now();
            self.last_ping = Some(now);
            self.unanswered_since.get_or_insert(now);
        }
        Ok(())
    }

    /// Every event that arrived, waiting briefly for the first; an error means the
    /// service is gone, or answered no ping for longer than the silence limit.
    pub fn read_events(&mut self) -> Result<Vec<SpottingEvent>> {
        let mut events = Vec::new();
        while let Some(message) = self.next_message()? {
            if let Message::Text(text) = message {
                events.extend(self.parse(text.as_str()).and_then(Answer::event));
            }
        }
        self.check_answering()?;
        Ok(events)
    }

    /// Fails when a ping has gone without any answer for longer than the silence limit — a
    /// service that hangs; called only after reading what arrived.
    fn check_answering(&self) -> Result<()> {
        match self.unanswered_since.map(|since| since.elapsed()) {
            Some(silent) if silent > SILENCE_LIMIT => Err(stopped_answering(silent)),
            _ => Ok(()),
        }
    }

    /// Every event for the audio sent so far, once the service has answered all of it: the
    /// service answers a sync only after the events for every message sent before it. A
    /// service that keeps answering pings is waited for up to `within`; a service that
    /// answers nothing fails the wait at the silence limit.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    pub fn events_for_sent_audio(&mut self, within: Duration) -> Result<Vec<SpottingEvent>> {
        self.answer_waits += 1;
        let id = self.answer_waits;
        self.socket
            .send(Message::text(serde_json::json!({ "sync": id }).to_string()))?;
        let deadline = Instant::now() + within;
        let mut events = Vec::new();
        loop {
            if deadline.checked_duration_since(Instant::now()).is_none() {
                bail!(
                    "the command spotting service did not answer sync {id}, the events for the \
                     audio sent, within {within:?}, though it answers pings"
                );
            }
            self.keep_alive()?;
            if let Some(Message::Text(text)) = self.next_message()? {
                match self.parse(text.as_str()) {
                    Some(Answer::Synced(synced)) if synced == id => return Ok(events),
                    Some(answer) => events.extend(answer.event()),
                    None => {}
                }
            }
            self.check_answering()?;
        }
    }

    pub fn close(mut self) {
        let _ = self.socket.close(None);
        let _ = self.socket.flush();
    }

    /// The next message of the service, or `None` when none arrived within the read wait;
    /// an error means the service is gone.
    fn next_message(&mut self) -> Result<Option<Message>> {
        match self.socket.read() {
            Ok(Message::Close(frame)) => bail!("the service closed the connection: {frame:?}"),
            Ok(message) => {
                self.unanswered_since = None;
                Ok(Some(message))
            }
            Err(tungstenite::Error::Io(e)) if is_timeout(&e) => Ok(None),
            Err(e) => Err(anyhow!(e).context("command spotting service connection lost")),
        }
    }

    fn parse(&self, text: &str) -> Option<Answer> {
        match serde_json::from_str::<ServiceEvent>(text) {
            Ok(ServiceEvent::Synced { id }) => Some(Answer::Synced(id)),
            Ok(event) => {
                to_spotting_event(event, self.audio_start, Instant::now()).map(Answer::Event)
            }
            Err(e) => {
                log::warn!("command spotting service sent an unknown event {text}: {e}");
                None
            }
        }
    }
}

/// A message of the service: a spotting event, or the answer to a sync.
enum Answer {
    Event(SpottingEvent),
    Synced(u64),
}

impl Answer {
    fn event(self) -> Option<SpottingEvent> {
        match self {
            Self::Event(event) => Some(event),
            Self::Synced(_) => None,
        }
    }
}

/// A service event with its times, seconds since the first sample, as instants from
/// `audio_start`, the moment that sample was captured; a phrase or command tells how its
/// report came about, which reached the app at `arrived_at`.
fn to_spotting_event(
    event: ServiceEvent,
    audio_start: Instant,
    arrived_at: Instant,
) -> Option<SpottingEvent> {
    let at = |secs: f32| audio_start + Duration::from_secs_f32(secs.max(0.0));
    let report = |end: f32, confirmation: Option<Confirmation>| {
        Some(ReportTiming {
            words_ended: at(end),
            confirmation: confirmation.map(|c| ConfirmationTiming {
                asked_at: at(c.asked_at),
                took: Duration::from_millis(c.took_ms),
            }),
            arrived_at,
        })
    };
    Some(match event {
        ServiceEvent::SpeechStart { t } => SpottingEvent::SpeechStarted(at(t)),
        ServiceEvent::SpeechEnd { t } => SpottingEvent::SpeechEnded(at(t)),
        ServiceEvent::Phrase { phrase, start, end, timing } => SpottingEvent::PhraseSpotted {
            phrase,
            start: at(start),
            end: at(end),
            timing: report(end, timing),
        },
        ServiceEvent::Command { commands, start, end, placement, timing } => {
            SpottingEvent::CommandsSaid {
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
                timing: report(end, timing),
            }
        }
        ServiceEvent::Synced { .. } => return None,
    })
}

/// A WebSocket connection to the service's own port, else to its first copy that
/// completes the handshake; the error of the own port when none does.
fn handshake_any(url: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let own = handshake(url);
    if own.is_ok() {
        return own;
    }
    let copies = ServiceAddress::parse(url)
        .map(|address| {
            let address = address.with_copy_offset(COMMAND_SPOTTING_COPY_OFFSET);
            address
                .copy_ports()
                .into_iter()
                .map(|port| address.with_port(port))
                .collect()
        })
        .unwrap_or_else(|_| Vec::new());
    copies
        .iter()
        .find_map(|copy| handshake(copy).ok())
        .map_or(own, Ok)
}

/// A WebSocket connection to `url`, given up after `CONNECT_WAIT` at each step.
fn handshake(url: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
    let parsed = url::Url::parse(url)?;
    let addrs = parsed.socket_addrs(|| None)?;
    let addr = addrs
        .first()
        .ok_or_else(|| anyhow!("no address for {url}"))?;
    let stream = TcpStream::connect_timeout(addr, CONNECT_WAIT)?;
    stream.set_read_timeout(Some(CONNECT_WAIT))?;
    stream.set_write_timeout(Some(CONNECT_WAIT))?;
    let (socket, _) = tungstenite::client(url, MaybeTlsStream::Plain(stream))
        .map_err(|e| anyhow!("WebSocket handshake: {e}"))?;
    Ok(socket)
}

/// Fails when nothing answers at `url`.
pub fn probe(url: &str) -> Result<()> {
    SpottingClient::connect(url, Instant::now())?.close();
    Ok(())
}

fn stopped_answering(silent: Duration) -> anyhow::Error {
    anyhow!(
        "command spotting service stopped answering ({}s without a reply)",
        silent.as_secs()
    )
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
        let event: ServiceEvent =
            serde_json::from_str(r#"{"event":"speech_start","t":-0.1}"#).unwrap();
        let audio_start = Instant::now();
        // When
        let converted = to_spotting_event(event, audio_start, audio_start);
        // Then
        assert_eq!(converted, Some(SpottingEvent::SpeechStarted(audio_start)));
    }

    #[test]
    fn should_read_speech_bounds() {
        let start: ServiceEvent =
            serde_json::from_str(r#"{"event":"speech_start","t":3.0}"#).unwrap();
        let end: ServiceEvent = serde_json::from_str(r#"{"event":"speech_end","t":4.5}"#).unwrap();
        assert!(matches!(start, ServiceEvent::SpeechStart { t } if t == 3.0));
        assert!(matches!(end, ServiceEvent::SpeechEnd { t } if t == 4.5));
    }

    #[test]
    fn should_read_the_answer_to_a_sync() {
        let synced: ServiceEvent = serde_json::from_str(r#"{"event":"synced","id":7}"#).unwrap();
        assert!(matches!(synced, ServiceEvent::Synced { id: 7 }));
    }
}
