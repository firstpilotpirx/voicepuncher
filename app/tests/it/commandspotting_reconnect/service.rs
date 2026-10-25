//! Stand-in command spotting services, the log capture and the connection records the tests read.

use super::*;
use voicepuncher_test_steps::log_lines::{captured, LogLines};

pub(super) const MIC_RATE_HZ: u32 = 48_000;

pub(super) const MIC_BUFFER_SECS: f32 = 0.1;

pub(super) const READ_WAIT: Duration = Duration::from_millis(10);

/// How long the stub takes to record a phrase selection: it stays, so a test that reads the
/// record right after an app event, without waiting for it, fails every run.
pub(super) const RECORD_LAG: Duration = Duration::from_millis(50);

/// The stub has recorded what the app sent before an app event well within this.
pub(super) const RECORD_WITHIN: Duration = Duration::from_secs(2);

/// How long the first connection lives before the stub closes it.
pub(super) const FIRST_CONNECTION: Duration = Duration::from_millis(500);

/// The loss is noticed and the service reached again well within this.
pub(super) const BACK_WITHIN: Duration = Duration::from_secs(5);

/// The long-outage test runs the client's reconnect pauses and replay this many times faster.
pub(super) const TIMINGS_SPEEDUP: u32 = 10;

/// Nothing listens on the port at least this long, past the replay of the client running
/// `TIMINGS_SPEEDUP` times faster, and until it has failed its tenth attempt, as a minute is
/// for the client at its own timings.
pub(super) const LONG_OUTAGE: Duration = Duration::from_millis(3_500);

/// The client's tenth attempt fails well within this, however busy the machine.
pub(super) const TENTH_ATTEMPT_WITHIN: Duration = Duration::from_secs(60);

/// A short outage: the stub refuses every connection for this long after it closed the first.
pub(super) const SHORT_OUTAGE: Duration = Duration::from_secs(3);

/// An outage the replay on the return no longer hides: well past its 5 s.
pub(super) const OUTAGE_PAST_REPLAY: Duration = Duration::from_secs(7);

/// How far the time of an event on the new connection may be from the capture of the first
/// sample sent again: the resampler's delay and a frame.
pub(super) const EVENT_TIME_TOLERANCE: Duration = Duration::from_millis(300);

pub(super) const SAMPLE_RATE_HZ: f32 = 16_000.0;

/// How loud the speech said during the outage is: the stub finds it in what it receives.
pub(super) const MARK: f32 = 0.5;

/// What a stub connection received: its phrase selections and its audio samples.
#[derive(Default)]
pub(super) struct Received {
    pub(crate) selections: Vec<String>,
    samples: Vec<f32>,
}

/// A stub service on `listener`: the first connection is closed after `FIRST_CONNECTION`,
/// every connection in the `away_for` after that is dropped at once, and every later one is
/// kept, what it receives recorded and its first audio answered with a speech start at 0 s.
pub(super) fn stub_service(listener: TcpListener, away_for: Duration) -> Arc<Mutex<Vec<Received>>> {
    stub_service_back_at(listener, away_for).0
}

/// As `stub_service`, with the moment it answers again, set once the first connection came.
pub(super) fn stub_service_back_at(
    listener: TcpListener,
    away_for: Duration,
) -> (Arc<Mutex<Vec<Received>>>, Arc<OnceLock<Instant>>) {
    let connections = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&connections);
    let back_at = Arc::new(OnceLock::new());
    let answers_at = Arc::clone(&back_at);
    std::thread::spawn(move || {
        for (n, stream) in listener.incoming().flatten().enumerate() {
            if n == 0 {
                let _ = answers_at.set(Instant::now() + FIRST_CONNECTION + away_for);
            } else if answers_at.get().is_some_and(|at| Instant::now() < *at) {
                drop(stream);
                continue;
            }
            let connections = Arc::clone(&recorded);
            std::thread::spawn(move || serve(stream, n, &connections));
        }
    });
    (connections, back_at)
}

pub(super) fn serve(stream: TcpStream, n: usize, connections: &Mutex<Vec<Received>>) {
    let Ok(mut socket) = tungstenite::accept(stream) else {
        return;
    };
    socket.get_ref().set_read_timeout(Some(READ_WAIT)).unwrap();
    let index = {
        let mut all = connections.lock().unwrap();
        all.push(Received::default());
        all.len() - 1
    };
    let opened = Instant::now();
    let mut answered = false;
    loop {
        if n == 0 && opened.elapsed() > FIRST_CONNECTION {
            let _ = socket.close(None);
            let _ = socket.flush();
            return;
        }
        if !record_next(&mut socket, index, connections) {
            return;
        }
        if n > 0 && !answered && !connections.lock().unwrap()[index].samples.is_empty() {
            answered = true;
            let _ = socket.send(Message::text(r#"{"event":"speech_start","t":0.0}"#));
        }
    }
}

/// Every log line, so a test can find the ones about its own service address.
pub(super) fn lines() -> &'static LogLines {
    captured(log::LevelFilter::Info)
}

/// The log lines that name `fragment`.
pub(super) fn logged(fragment: &str) -> Vec<String> {
    lines().containing(fragment)
}

/// Records the next message of `socket`; false once the connection ended.
pub(super) fn record_next(
    socket: &mut WebSocket<TcpStream>,
    index: usize,
    connections: &Mutex<Vec<Received>>,
) -> bool {
    match socket.read() {
        Ok(Message::Text(text)) => {
            std::thread::sleep(RECORD_LAG);
            connections.lock().unwrap()[index]
                .selections
                .push(text.to_string());
        }
        Ok(Message::Binary(bytes)) => connections.lock().unwrap()[index].samples.extend(
            bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        ),
        Ok(_) => {}
        Err(e) if crate::fake_websocket::nothing_yet(&e) => {}
        Err(_) => return false,
    }
    true
}

/// A service that serves one connection, closes it after `FIRST_CONNECTION` and then no
/// longer listens on its port.
pub(super) fn serve_one_connection_then_go(listener: TcpListener) {
    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        drop(listener);
        serve(stream, 0, &Mutex::new(Vec::new()));
    });
}

pub(super) use crate::fake_websocket::free_listener;

/// How many samples of the speech said during the outage `connection` received.
pub(super) fn marked(connection: &Received) -> usize {
    connection
        .samples
        .iter()
        .filter(|s| (**s - MARK).abs() < 0.1)
        .count()
}

/// Waits, at most `RECORD_WITHIN`, until what the stub connections received holds `holds`:
/// the stub records on its own thread, so it may lag behind the app event a test saw.
pub(super) fn wait_for_record(
    connections: &Mutex<Vec<Received>>,
    holds: impl Fn(&[Received]) -> bool,
) {
    let deadline = Instant::now() + RECORD_WITHIN;
    while Instant::now() < deadline && !holds(&connections.lock().unwrap()) {
        std::thread::sleep(READ_WAIT);
    }
}

/// The time of the next speech start the app is told of, feeding silence meanwhile.
pub(super) fn next_speech_start(
    audio: &Sender<Vec<f32>>,
    inputs: &Receiver<PipelineInput>,
) -> Option<Instant> {
    let deadline = Instant::now() + BACK_WITHIN;
    while Instant::now() < deadline {
        audio.send(microphone(0.0)).unwrap();
        if let Ok(PipelineInput::Spotting(SpottingEvent::SpeechStarted(at))) =
            inputs.recv_timeout(Duration::from_millis(100))
        {
            return Some(at);
        }
    }
    None
}
