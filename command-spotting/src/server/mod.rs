//! Serves each WebSocket connection with two threads: one owns the socket, takes the app's
//! audio and answers its pings at once; the other spots commands in that audio.
//!
//! The confirmations by the speech recognition service run on a third (see `worker`), so a slow one
//! never keeps the connection from answering.

use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tungstenite::{Message, WebSocket};

use crate::engine::{Engine, Session};
use crate::health::{Beat, Health};
use crate::protocol::{samples_from_bytes, AppMessage, PhraseSelection, ServerEvent};

/// How long a read on the socket waits before the I/O thread sends what is ready.
const READ_WAIT: Duration = Duration::from_millis(10);
/// How long the processing thread waits for input before it applies the answers that came.
const POLL_WAIT: Duration = Duration::from_millis(20);

mod probe;
mod refusal;
mod stats;

use probe::{answer_health, is_health_probe};
use refusal::{greet, refuse, refuse_or_end, Refusal};
use stats::Stats;

static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

enum Input {
    Hello(u32),
    Audio(Vec<f32>, Instant),
    Select(PhraseSelection),
    Sync(u64),
}

enum Output {
    Event(String),
    Refuse(String),
}

pub fn serve(listener: &TcpListener, engine: &Arc<Engine>) -> Result<()> {
    let health = Health::default();
    for stream in listener.incoming() {
        let stream = stream?;
        let conn = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
        let engine = Arc::clone(engine);
        let health = health.clone();
        std::thread::Builder::new()
            .name(format!("conn-{conn}-io"))
            .spawn(move || {
                if let Err(e) = serve_connection(stream, &engine, conn, &health) {
                    log::warn!("conn={conn} ended: {e:#}");
                }
            })?;
    }
    Ok(())
}

fn serve_connection(stream: TcpStream, engine: &Engine, conn: u64, health: &Health) -> Result<()> {
    if is_health_probe(&stream) {
        return answer_health(stream, health);
    }
    let mut socket = match tungstenite::accept(stream) {
        Ok(socket) => socket,
        Err(e) => {
            log::debug!("conn={conn} not a WebSocket client (a port probe?): {e}");
            return Ok(());
        }
    };
    socket.get_ref().set_read_timeout(Some(READ_WAIT))?;
    let session = engine.session_for(conn)?;
    let beat = health.register(conn);
    log::info!("conn={conn} app connected");
    let (inputs, input_rx) = mpsc::channel();
    let (output_tx, outputs) = mpsc::channel();
    let ended = std::thread::scope(|scope| {
        let processing_beat = &beat;
        std::thread::Builder::new()
            .name(format!("conn-{conn}-proc"))
            .spawn_scoped(scope, move || {
                process(session, &input_rx, &output_tx, conn, processing_beat);
            })?;
        exchange(&mut socket, &inputs, &outputs, conn, &beat)
    });
    health.unregister(conn);
    ended
}

/// The I/O thread: the app's messages go to the processing thread, its events to the app.
fn exchange(
    socket: &mut WebSocket<TcpStream>,
    inputs: &Sender<Input>,
    outputs: &Receiver<Output>,
    conn: u64,
    beat: &Beat,
) -> Result<()> {
    let mut greeted = false;
    loop {
        let input = read_input(socket, conn, greeted).and_then(|i| greet(i, &mut greeted));
        let input = match input {
            Ok(input) => input,
            Err(e) => return refuse_or_end(socket, conn, e),
        };
        if let Some(input) = input {
            forward(socket, inputs, input, conn, beat)?;
        }
        if let Some(ended) = send_outputs(socket, outputs, conn) {
            return ended;
        }
    }
}

/// Hands `input` to the processing thread, or drops audio while it is behind.
fn forward(
    socket: &mut WebSocket<TcpStream>,
    inputs: &Sender<Input>,
    input: Input,
    conn: u64,
    beat: &Beat,
) -> Result<()> {
    match input {
        Input::Audio(samples, _) if !beat.queued() => drop_audio(socket, conn, samples.len()),
        input => inputs
            .send(input)
            .map_err(|_| anyhow!("the processing thread ended")),
    }
}

/// Sends the processing thread's events to the app; how the connection ends, once it does.
fn send_outputs(
    socket: &mut WebSocket<TcpStream>,
    outputs: &Receiver<Output>,
    conn: u64,
) -> Option<Result<()>> {
    loop {
        let sent = match outputs.try_recv() {
            Ok(Output::Event(json)) => socket.send(Message::text(json)),
            Ok(Output::Refuse(reason)) => {
                return Some(refuse(socket, conn, &Refusal::policy(reason)))
            }
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                return Some(Err(anyhow!("the processing thread ended")))
            }
        };
        if let Err(e) = sent {
            return Some(Err(e.into()));
        }
    }
}

/// The next message of the app, or none within the read wait; `Ok(None)` also for a ping,
/// which tungstenite answers on the next read or write. A message that breaks the protocol
/// fails with the [`Refusal`] to close the connection with.
fn read_input(
    socket: &mut WebSocket<TcpStream>,
    conn: u64,
    greeted: bool,
) -> Result<Option<Input>> {
    match socket.read() {
        Ok(Message::Text(text)) => text_input(text.as_str(), greeted).map(Some),
        Ok(Message::Binary(bytes)) => match samples_from_bytes(&bytes) {
            Ok(samples) => Ok(Some(Input::Audio(samples, Instant::now()))),
            Err(reason) => Err(Refusal::invalid(reason).into()),
        },
        Ok(Message::Close(_)) => {
            log::info!("conn={conn} app disconnected");
            Err(anyhow!("the app closed the connection"))
        }
        Ok(_) => Ok(None),
        Err(tungstenite::Error::Io(e))
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

/// The app's text message; one that cannot be read breaks the protocol.
fn text_input(text: &str, greeted: bool) -> Result<Input> {
    Ok(match serde_json::from_str::<AppMessage>(text) {
        Ok(AppMessage::Hello { version }) => Input::Hello(version),
        Ok(AppMessage::Select(selection)) => Input::Select(selection),
        Ok(AppMessage::Sync { id }) => Input::Sync(id),
        Err(_) if !greeted => return Err(Refusal::no_hello().into()),
        Err(e) => return Err(Refusal::invalid(format!("malformed message: {e}")).into()),
    })
}

/// The processing thread: spots and detects speech in the audio, in the order it came.
fn process(
    mut session: Session<'_>,
    inputs: &Receiver<Input>,
    outputs: &Sender<Output>,
    conn: u64,
    beat: &Beat,
) {
    let mut stats = Stats::new();
    let mut reported = false;
    loop {
        let mut events = match inputs.recv_timeout(POLL_WAIT) {
            Ok(input) => match handle(&mut session, input, &mut stats, conn, beat) {
                Ok(events) => events,
                Err(unknown) => {
                    let _ = outputs.send(Output::Refuse(unknown));
                    return;
                }
            },
            Err(RecvTimeoutError::Timeout) => session.poll(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        beat.alive();
        events.extend(watch_stall(session.stalled(), beat, conn, &mut reported));
        if !send_events(outputs, &events) {
            return;
        }
        stats.maybe_log(conn, session.pending_confirmations());
    }
}

/// The events `input` gives; the refusal's reason for a selection of unknown phrases.
fn handle(
    session: &mut Session<'_>,
    input: Input,
    stats: &mut Stats,
    conn: u64,
    beat: &Beat,
) -> Result<Vec<ServerEvent>, String> {
    match input {
        Input::Audio(samples, received) => {
            stats.audio(received.elapsed(), conn);
            let events = session.accept(&samples);
            beat.processed();
            Ok(events)
        }
        Input::Hello(_) => Ok(Vec::new()),
        Input::Select(selection) => {
            session
                .select(&selection)
                .map_err(|unknown| unknown.to_string())?;
            log::info!("conn={conn} listening for {:?}", selection.phrases);
            Ok(Vec::new())
        }
        Input::Sync(id) => {
            log::debug!("conn={conn} sync={id} asked");
            Ok(session.sync(id))
        }
    }
}

/// Sends `events` to the I/O thread; false once it ended.
fn send_events(outputs: &Sender<Output>, events: &[ServerEvent]) -> bool {
    events
        .iter()
        .filter_map(|event| serde_json::to_string(event).ok())
        .all(|json| outputs.send(Output::Event(json)).is_ok())
}

/// Keeps the connection's health current with its confirmations, and tells the app once when
/// one runs past its attempts.
fn watch_stall(
    stalled: Option<Duration>,
    beat: &Beat,
    conn: u64,
    reported: &mut bool,
) -> Option<ServerEvent> {
    beat.stalled(stalled.is_some());
    let first = stalled.filter(|_| !*reported);
    *reported = stalled.is_some();
    let waited = first?;
    log::warn!(
        "conn={conn} a confirmation has run {}ms, past its attempts: stalled",
        waited.as_millis()
    );
    let waited_ms = u64::try_from(waited.as_millis()).unwrap_or(u64::MAX);
    Some(ServerEvent::Stalled { waited_ms })
}

/// Drops audio that does not fit the backlog, and tells the app.
fn drop_audio(socket: &mut WebSocket<TcpStream>, conn: u64, samples: usize) -> Result<()> {
    log::warn!("conn={conn} dropped {samples} samples: audio has waited past 10 s");
    let dropped = ServerEvent::Dropped { samples: samples as u64 };
    socket.send(Message::text(serde_json::to_string(&dropped)?))?;
    Ok(())
}

#[cfg(test)]
mod tests;
