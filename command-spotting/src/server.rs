//! Serves each WebSocket connection with two threads: one owns the socket, takes the app's
//! audio and answers its pings at once; the other spots commands in that audio. The
//! confirmations by the speech recognition service run on a third (see `worker`), so a
//! slow one never keeps the connection from answering.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::{Message, WebSocket};

use crate::engine::{Engine, Session};
use crate::health::{Beat, Health};
use crate::protocol::{samples_from_bytes, AppMessage};

/// How long a read on the socket waits before the I/O thread sends what is ready.
const READ_WAIT: Duration = Duration::from_millis(10);
/// How long the processing thread waits for input before it applies the answers that came.
const POLL_WAIT: Duration = Duration::from_millis(20);
/// Audio that waited longer than this for the processing thread is logged as lag.
const LAG_LOGGED: Duration = Duration::from_secs(1);
/// How often a connection logs its counters.
const STATS_EVERY: Duration = Duration::from_secs(30);
/// How long the accept thread waits for a health probe's request line.
const PROBE_WAIT: Duration = Duration::from_secs(2);
const HEALTH_REQUEST: &[u8] = b"GET /health";

static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

enum Input {
    Audio(Vec<f32>, Instant),
    Select(Vec<String>),
    Sync(u64),
}

enum Output {
    Event(String),
    Refuse(String),
}

pub fn serve(listener: TcpListener, engine: Arc<Engine>) -> Result<()> {
    let health = Health::default();
    for stream in listener.incoming() {
        let stream = stream?;
        let conn = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
        let engine = Arc::clone(&engine);
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
        std::thread::Builder::new()
            .name(format!("conn-{conn}-proc"))
            .spawn_scoped(scope, || process(session, input_rx, output_tx, conn, &beat))?;
        exchange(&mut socket, inputs, outputs, conn, &beat)
    });
    health.unregister(conn);
    ended
}

/// The I/O thread: the app's messages go to the processing thread, its events to the app.
fn exchange(
    socket: &mut WebSocket<TcpStream>,
    inputs: Sender<Input>,
    outputs: Receiver<Output>,
    conn: u64,
    beat: &Beat,
) -> Result<()> {
    loop {
        if let Some(input) = read_input(socket, conn)? {
            if matches!(input, Input::Audio(..)) {
                beat.queued();
            }
            inputs
                .send(input)
                .map_err(|_| anyhow!("the processing thread ended"))?;
        }
        loop {
            match outputs.try_recv() {
                Ok(Output::Event(json)) => socket.send(Message::text(json))?,
                Ok(Output::Refuse(reason)) => return refuse(socket, conn, &reason),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    return Err(anyhow!("the processing thread ended"))
                }
            }
        }
    }
}

/// The next message of the app, or none within the read wait; `Ok(None)` also for a ping,
/// which tungstenite answers on the next read or write.
fn read_input(socket: &mut WebSocket<TcpStream>, conn: u64) -> Result<Option<Input>> {
    match socket.read() {
        Ok(Message::Text(text)) => match serde_json::from_str::<AppMessage>(text.as_str())? {
            AppMessage::Select(selection) => Ok(Some(Input::Select(selection.phrases))),
            AppMessage::Sync { sync } => Ok(Some(Input::Sync(sync))),
        },
        Ok(Message::Binary(bytes)) => Ok(Some(Input::Audio(
            samples_from_bytes(&bytes),
            Instant::now(),
        ))),
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

/// The processing thread: spots and detects speech in the audio, in the order it came.
fn process(
    mut session: Session<'_>,
    inputs: Receiver<Input>,
    outputs: Sender<Output>,
    conn: u64,
    beat: &Beat,
) {
    let mut stats = Stats::new();
    loop {
        let events = match inputs.recv_timeout(POLL_WAIT) {
            Ok(Input::Audio(samples, received)) => {
                stats.audio(received.elapsed(), conn);
                let events = session.accept(&samples);
                beat.processed();
                events
            }
            Ok(Input::Select(phrases)) => match session.listen_for(&phrases) {
                Ok(()) => {
                    log::info!("conn={conn} listening for {phrases:?}");
                    Vec::new()
                }
                Err(unknown) => {
                    let _ = outputs.send(Output::Refuse(unknown.to_string()));
                    return;
                }
            },
            Ok(Input::Sync(id)) => {
                log::debug!("conn={conn} sync={id} asked");
                session.sync(id)
            }
            Err(RecvTimeoutError::Timeout) => session.poll(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        beat.alive();
        for event in events {
            let Ok(json) = serde_json::to_string(&event) else {
                continue;
            };
            if outputs.send(Output::Event(json)).is_err() {
                return;
            }
        }
        stats.maybe_log(conn, session.pending_confirmations());
    }
}

/// Counters a connection logs every `STATS_EVERY`.
struct Stats {
    since: Instant,
    messages: u64,
    worst_lag: Duration,
}

impl Stats {
    fn new() -> Self {
        Self {
            since: Instant::now(),
            messages: 0,
            worst_lag: Duration::ZERO,
        }
    }

    fn audio(&mut self, waited: Duration, conn: u64) {
        self.messages += 1;
        self.worst_lag = self.worst_lag.max(waited);
        if waited > LAG_LOGGED {
            log::info!(
                "conn={conn} lag={}ms: audio waited for processing",
                waited.as_millis()
            );
        }
    }

    fn maybe_log(&mut self, conn: u64, pending: usize) {
        if self.since.elapsed() < STATS_EVERY {
            return;
        }
        log::info!(
            "stats conn={conn} messages={} worst_lag={}ms confirmations_pending={pending}",
            self.messages,
            self.worst_lag.as_millis()
        );
        *self = Self::new();
    }
}

/// A plain HTTP `GET /health` on the WebSocket port, told apart by its request line.
fn is_health_probe(stream: &TcpStream) -> bool {
    let mut line = [0u8; HEALTH_REQUEST.len()];
    let _ = stream.set_read_timeout(Some(PROBE_WAIT));
    let started = Instant::now();
    while started.elapsed() < PROBE_WAIT {
        match stream.peek(&mut line) {
            Ok(n) if n >= line.len() => return line == HEALTH_REQUEST,
            Ok(n) if n > 0 && !HEALTH_REQUEST.starts_with(&line[..n]) => return false,
            Ok(0) | Err(_) => return false,
            Ok(_) => std::thread::sleep(READ_WAIT),
        }
    }
    false
}

fn answer_health(mut stream: TcpStream, health: &Health) -> Result<()> {
    let mut request = [0u8; 1024];
    let _ = stream.read(&mut request);
    let (status, body) = match health.stuck() {
        None => ("200 OK", "ok\n".to_owned()),
        Some(conn) => {
            log::warn!("health: conn={conn} has audio waiting and made no progress");
            ("503 Service Unavailable", format!("stuck conn={conn}\n"))
        }
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

fn refuse(socket: &mut WebSocket<TcpStream>, conn: u64, reason: &str) -> Result<()> {
    log::warn!("conn={conn} refusing the connection: {reason}");
    socket.close(Some(CloseFrame {
        code: CloseCode::Policy,
        reason: reason.to_owned().into(),
    }))?;
    while socket.read().is_ok() {}
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    use super::*;

    /// What the service answers to a health probe while `health` holds.
    fn health_answer(health: &Health) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut probe = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        probe
            .write_all(b"GET /health HTTP/1.1\r\nHost: x\r\n\r\n")
            .unwrap();
        let (stream, _) = listener.accept().unwrap();
        assert!(is_health_probe(&stream));
        answer_health(stream, health).unwrap();
        let mut answer = String::new();
        probe.read_to_string(&mut answer).unwrap();
        answer
    }

    #[test]
    fn should_answer_ok_without_connections() {
        assert!(health_answer(&Health::default()).starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn should_answer_ok_while_every_connection_keeps_up() {
        // Given
        let health = Health::default();
        let beat = health.register(3);
        // When
        beat.queued();
        // Then
        assert!(health_answer(&health).starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn should_answer_unavailable_naming_a_stuck_connection() {
        // Given a connection with audio waiting and no progress since
        let health = Health::stuck_after(Duration::ZERO);
        let beat = health.register(3);
        beat.queued();
        std::thread::sleep(Duration::from_millis(5));
        // When
        let answer = health_answer(&health);
        // Then
        assert!(answer.starts_with("HTTP/1.1 503"), "{answer}");
        assert!(answer.contains("stuck conn=3"), "{answer}");
    }

    #[test]
    fn should_not_take_a_websocket_handshake_for_a_health_probe() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        client
            .write_all(b"GET / HTTP/1.1\r\nUpgrade: websocket\r\n\r\n")
            .unwrap();
        let (stream, _) = listener.accept().unwrap();
        assert!(!is_health_probe(&stream));
    }
}
