//! The app's side of the command spotting service protocol: phrase selection and audio
//! out, spotting events in. Service times are seconds since the first sample sent; the
//! client turns them into instants from the moment that sample was captured.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::core::domain::{SpottingEvent, VoiceCommand};

mod events;

use events::{to_spotting_event, ServiceEvent};

mod handshake;

use handshake::handshake_any;

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
/// The version of the command spotting protocol the client speaks, said in its `hello`.
const PROTOCOL_VERSION: u32 = 2;

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
        let mut socket = handshake_any(url).with_context(|| {
            format!(
                "command spotting service not reachable at {url} — run 'just start-model-servers'"
            )
        })?;
        if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
            stream.set_read_timeout(Some(READ_WAIT))?;
            stream.set_write_timeout(Some(WRITE_WAIT))?;
        }
        let hello = serde_json::json!({ "type": "hello", "version": PROTOCOL_VERSION });
        socket.send(Message::text(hello.to_string()))?;
        Ok(Self { socket, audio_start, unanswered_since: None, last_ping: None, answer_waits: 0 })
    }

    /// Spots the phrases of `commands` from now on.
    pub fn listen_for(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        self.socket.send(Message::text(selection_of(commands)))?;
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
                events.extend(self.parse(text.as_str())?.and_then(Answer::event));
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
        self.socket.send(Message::text(
            serde_json::json!({ "type": "sync", "id": id }).to_string(),
        ))?;
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
                match self.parse(text.as_str())? {
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

    /// The service's message; an error when the service says the connection is to be left.
    fn parse(&self, text: &str) -> Result<Option<Answer>> {
        Ok(match serde_json::from_str::<ServiceEvent>(text) {
            Ok(ServiceEvent::Synced { id }) => Some(Answer::Synced(id)),
            Ok(ServiceEvent::Stalled { waited_ms }) => {
                bail!("the command spotting service stalled: a confirmation has run {waited_ms} ms")
            }
            Ok(ServiceEvent::Dropped { samples }) => {
                bail!("the command spotting service dropped {samples} samples it could not keep up with")
            }
            Ok(event) => {
                to_spotting_event(event, self.audio_start, Instant::now()).map(Answer::Event)
            }
            Err(e) => {
                log::warn!("command spotting service sent an unknown event {text}: {e}");
                None
            }
        })
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

/// The `select` message for `commands`: the start of an utterance is searched early while
/// "start dictation" is among them.
fn selection_of(commands: &[VoiceCommand]) -> String {
    let phrases: Vec<&str> = commands.iter().map(|c| c.phrase()).collect();
    let early_start = commands.contains(&VoiceCommand::StartDictation);
    serde_json::json!({ "type": "select", "phrases": phrases, "early_start": early_start })
        .to_string()
}

#[cfg(test)]
mod tests;
