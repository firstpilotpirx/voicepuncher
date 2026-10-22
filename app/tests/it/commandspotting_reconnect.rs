//! The command spotting adapter against stub services on a loopback port that close the
//! connection, go away and come back: the adapter reports the loss once, an outage past the
//! replay once, reaches for the service until it answers, reports its return and sends it the
//! audio it missed.
//! A service that accepts but never completes the handshake fails an attempt in seconds.

use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tungstenite::{Message, WebSocket};

use voicepuncher::adapters::secondary::commandspotting::websocket::{AudioStream, LinkTimings};
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::{SpottingEvent, VoiceCommand};

const MIC_RATE_HZ: u32 = 48_000;
const MIC_BUFFER_SECS: f32 = 0.1;
const READ_WAIT: Duration = Duration::from_millis(10);
/// How long the stub takes to record a phrase selection: it stays, so a test that reads the
/// record right after an app event, without waiting for it, fails every run.
const RECORD_LAG: Duration = Duration::from_millis(50);
/// The stub has recorded what the app sent before an app event well within this.
const RECORD_WITHIN: Duration = Duration::from_secs(2);
/// How long the first connection lives before the stub closes it.
const FIRST_CONNECTION: Duration = Duration::from_millis(500);
/// The loss is noticed and the service reached again well within this.
const BACK_WITHIN: Duration = Duration::from_secs(5);
/// The long-outage test runs the client's reconnect pauses and replay this many times faster.
const TIMINGS_SPEEDUP: u32 = 10;
/// Nothing listens on the port at least this long, past the replay of the client running
/// `TIMINGS_SPEEDUP` times faster, and until it has failed its tenth attempt, as a minute is
/// for the client at its own timings.
const LONG_OUTAGE: Duration = Duration::from_millis(3_500);
/// The client's tenth attempt fails well within this, however busy the machine.
const TENTH_ATTEMPT_WITHIN: Duration = Duration::from_secs(60);
/// A short outage: the stub refuses every connection for this long after it closed the first.
const SHORT_OUTAGE: Duration = Duration::from_secs(3);
/// An outage the replay on the return no longer hides: well past its 5 s.
const OUTAGE_PAST_REPLAY: Duration = Duration::from_secs(7);
/// How far the time of an event on the new connection may be from the capture of the first
/// sample sent again: the resampler's delay and a frame.
const EVENT_TIME_TOLERANCE: Duration = Duration::from_millis(300);
const SAMPLE_RATE_HZ: f32 = 16_000.0;
/// How loud the speech said during the outage is: the stub finds it in what it receives.
const MARK: f32 = 0.5;

/// What a stub connection received: its phrase selections and its audio samples.
#[derive(Default)]
struct Received {
    selections: Vec<String>,
    samples: Vec<f32>,
}

/// A stub service on `listener`: the first connection is closed after `FIRST_CONNECTION`,
/// every connection in the `away_for` after that is dropped at once, and every later one is
/// kept, what it receives recorded and its first audio answered with a speech start at 0 s.
fn stub_service(listener: TcpListener, away_for: Duration) -> Arc<Mutex<Vec<Received>>> {
    stub_service_back_at(listener, away_for).0
}

/// As `stub_service`, with the moment it answers again, set once the first connection came.
fn stub_service_back_at(
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

fn serve(stream: TcpStream, n: usize, connections: &Mutex<Vec<Received>>) {
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

/// Keeps every log line, so a test can find the ones about its own service address.
struct Lines(Mutex<Vec<String>>);

impl log::Log for Lines {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        self.0.lock().unwrap().push(record.args().to_string());
    }
    fn flush(&self) {}
}

fn lines() -> &'static Lines {
    static LINES: OnceLock<&'static Lines> = OnceLock::new();
    LINES.get_or_init(|| {
        let lines: &'static Lines = Box::leak(Box::new(Lines(Mutex::default())));
        log::set_logger(lines).unwrap();
        log::set_max_level(log::LevelFilter::Info);
        lines
    })
}

/// The log lines that name `fragment`.
fn logged(fragment: &str) -> Vec<String> {
    lines()
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|l| l.contains(fragment))
        .cloned()
        .collect()
}

/// Records the next message of `socket`; false once the connection ended.
fn record_next(
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
        Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(_) => return false,
    }
    true
}

/// A service that serves one connection, closes it after `FIRST_CONNECTION` and then no
/// longer listens on its port.
fn serve_one_connection_then_go(listener: TcpListener) {
    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        drop(listener);
        serve(stream, 0, &Mutex::new(Vec::new()));
    });
}

fn free_listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (listener, url)
}

fn start_stream(url: &str) -> (AudioStream, Sender<Vec<f32>>, Receiver<PipelineInput>) {
    start_stream_from(url, Instant::now())
}

/// The stream of a client whose reconnect pauses and replay run `TIMINGS_SPEEDUP` times faster.
fn start_fast_stream(url: &str) -> (AudioStream, Sender<Vec<f32>>, Receiver<PipelineInput>) {
    let (audio_tx, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start_with_timings(
        url,
        &VoiceCommand::STANDBY,
        Instant::now(),
        MIC_RATE_HZ,
        audio_rx,
        inputs_tx,
        LinkTimings::default().faster(TIMINGS_SPEEDUP),
    )
    .unwrap();
    (stream, audio_tx, inputs_rx)
}

/// The stream of a microphone whose first sample was captured at `audio_start`.
fn start_stream_from(
    url: &str,
    audio_start: Instant,
) -> (AudioStream, Sender<Vec<f32>>, Receiver<PipelineInput>) {
    let (audio_tx, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs_rx) = mpsc::channel();
    let stream = AudioStream::start(
        url,
        &VoiceCommand::STANDBY,
        audio_start,
        MIC_RATE_HZ,
        audio_rx,
        inputs_tx,
    )
    .unwrap();
    (stream, audio_tx, inputs_rx)
}

fn microphone(level: f32) -> Vec<f32> {
    vec![level; (MIC_BUFFER_SECS * MIC_RATE_HZ as f32) as usize]
}

/// Feeds the microphone `level` until `input` arrives, at most for `within`.
fn feed_until(
    audio: &Sender<Vec<f32>>,
    inputs: &Receiver<PipelineInput>,
    level: f32,
    input: &PipelineInput,
    within: Duration,
) -> bool {
    let deadline = Instant::now() + within;
    while Instant::now() < deadline {
        audio.send(microphone(level)).unwrap();
        if inputs
            .recv_timeout(Duration::from_millis(100))
            .is_ok_and(|got| got == *input)
        {
            return true;
        }
    }
    false
}

#[test]
fn should_reach_for_the_service_again_after_it_closes_the_connection() {
    // Given
    let (listener, url) = free_listener();
    let connections = stub_service(listener, Duration::ZERO);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let lost = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingLost,
        BACK_WITHIN,
    );
    let back = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingBack,
        BACK_WITHIN,
    );
    // Then
    assert!(lost && back, "lost: {lost}, back: {back}");
    wait_for_record(&connections, |all| {
        all.get(1)
            .is_some_and(|c| c.selections.iter().any(|s| s.contains("press enter")))
    });
    let selections = &connections.lock().unwrap()[1].selections;
    assert!(
        selections.iter().any(|s| s.contains("press enter")),
        "{selections:?}"
    );
    stream.stop();
}

#[test]
#[expect(clippy::too_many_lines, reason = "size baseline")]
fn should_be_back_within_seconds_once_the_service_answers_after_a_long_outage() {
    // Given the service goes away and nothing listens on its port for a while
    lines();
    let (listener, url) = free_listener();
    let port_addr = listener.local_addr().unwrap();
    serve_one_connection_then_go(listener);
    let (stream, audio, inputs) = start_fast_stream(&url);
    let lost = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingLost,
        BACK_WITHIN,
    );
    let attempts = || logged(&format!("to reach {url} failed"));
    let tenth_failed = || attempts().iter().any(|l| l.contains("attempt 10 "));
    let away_since = Instant::now();
    let mut while_away = Vec::new();
    while (away_since.elapsed() < LONG_OUTAGE || !tenth_failed())
        && away_since.elapsed() < TENTH_ATTEMPT_WITHIN
    {
        audio.send(microphone(0.0)).unwrap();
        if let Ok(input) = inputs.recv_timeout(Duration::from_millis(100)) {
            while_away.push(input);
        }
    }
    // When the service answers on its port again, once the client failed its tenth attempt
    let _connections = stub_service(TcpListener::bind(port_addr).unwrap(), Duration::ZERO);
    let started = Instant::now();
    let back = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingBack,
        BACK_WITHIN,
    );
    // Then
    let client_log = logged(&url);
    assert!(lost, "the loss was not reported: {client_log:#?}");
    assert_eq!(
        while_away,
        [PipelineInput::SpottingStillAway],
        "only the long outage comes while away: {client_log:#?}"
    );
    assert!(
        back,
        "not back within {BACK_WITHIN:?}, waited {:?}: {client_log:#?}",
        started.elapsed()
    );
    let attempts = attempts();
    assert!(
        attempts.iter().any(|l| l.contains("attempt 1 ")),
        "{attempts:?}"
    );
    assert!(
        attempts.iter().any(|l| l.contains("attempt 10 ")),
        "{attempts:?}"
    );
    assert!(
        attempts.iter().all(|l| l.contains("session=")),
        "{attempts:?}"
    );
    stream.stop();
}

#[test]
fn should_report_once_that_the_service_is_still_away_past_the_replay() {
    // Given the service is away for longer than the replay reaches back
    lines();
    let (listener, url) = free_listener();
    let (_connections, back_at) = stub_service_back_at(listener, OUTAGE_PAST_REPLAY);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let outage = outage_inputs(&audio, &inputs, &back_at);
    // Then
    assert_eq!(
        outage,
        [
            PipelineInput::SpottingLost,
            PipelineInput::SpottingStillAway,
            PipelineInput::SpottingBack
        ],
        "{:#?}",
        logged(&url)
    );
    stream.stop();
}

#[test]
fn should_not_report_a_service_back_within_the_replay_as_still_away() {
    // Given the service is away for less than the replay reaches back
    lines();
    let (listener, url) = free_listener();
    let (_connections, back_at) = stub_service_back_at(listener, SHORT_OUTAGE);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let outage = outage_inputs(&audio, &inputs, &back_at);
    // Then
    assert_eq!(
        outage,
        [PipelineInput::SpottingLost, PipelineInput::SpottingBack],
        "{:#?}",
        logged(&url)
    );
    stream.stop();
}

/// The inputs other than spotting events, feeding silence until the service is back, at
/// most until `BACK_WITHIN` after the stub answers again (`back_at`).
fn outage_inputs(
    audio: &Sender<Vec<f32>>,
    inputs: &Receiver<PipelineInput>,
    back_at: &OnceLock<Instant>,
) -> Vec<PipelineInput> {
    let started = Instant::now();
    let waiting = || match back_at.get() {
        Some(at) => Instant::now() < *at + BACK_WITHIN,
        // The client has not connected yet: its first connection starts the outage.
        None => started.elapsed() < BACK_WITHIN,
    };
    let mut got = Vec::new();
    while waiting() && got.last() != Some(&PipelineInput::SpottingBack) {
        audio.send(microphone(0.0)).unwrap();
        match inputs.recv_timeout(Duration::from_millis(100)) {
            Ok(PipelineInput::Spotting(_)) | Err(_) => {}
            Ok(input) => got.push(input),
        }
    }
    got
}

#[test]
fn should_send_the_service_that_is_back_the_speech_heard_while_it_was_away() {
    // Given the service is away for a few seconds after the first connection
    let (listener, url) = free_listener();
    let connections = stub_service(listener, SHORT_OUTAGE);
    let audio_start = Instant::now();
    let (stream, audio, inputs) = start_stream_from(&url, audio_start);
    // When speech is heard between the loss and the return
    let lost = feed_until(
        &audio,
        &inputs,
        0.0,
        &PipelineInput::SpottingLost,
        BACK_WITHIN,
    );
    let back = feed_until(
        &audio,
        &inputs,
        MARK,
        &PipelineInput::SpottingBack,
        SHORT_OUTAGE + BACK_WITHIN,
    );
    let speech_start = next_speech_start(&audio, &inputs);
    // Then the speech reached the new connection, its events placed where it was said
    assert!(lost && back, "lost: {lost}, back: {back}");
    let outage_samples = ((SHORT_OUTAGE.as_secs_f32() - 1.0) * SAMPLE_RATE_HZ) as usize;
    wait_for_record(&connections, |all| {
        all.last().is_some_and(|c| marked(c) > outage_samples)
    });
    let marked = marked(connections.lock().unwrap().last().unwrap());
    assert!(
        marked > outage_samples,
        "the speech heard while away was not sent: {marked} samples"
    );
    let at = speech_start.expect("the new connection's event reached the app");
    assert!(
        at.saturating_duration_since(audio_start) < EVENT_TIME_TOLERANCE,
        "an event at the first sample sent again is placed {:?} after the capture of that sample",
        at.saturating_duration_since(audio_start)
    );
    stream.stop();
}

/// How many samples of the speech said during the outage `connection` received.
fn marked(connection: &Received) -> usize {
    connection
        .samples
        .iter()
        .filter(|s| (**s - MARK).abs() < 0.1)
        .count()
}

/// Waits, at most `RECORD_WITHIN`, until what the stub connections received holds `holds`:
/// the stub records on its own thread, so it may lag behind the app event a test saw.
fn wait_for_record(connections: &Mutex<Vec<Received>>, holds: impl Fn(&[Received]) -> bool) {
    let deadline = Instant::now() + RECORD_WITHIN;
    while Instant::now() < deadline && !holds(&connections.lock().unwrap()) {
        std::thread::sleep(READ_WAIT);
    }
}

/// The time of the next speech start the app is told of, feeding silence meanwhile.
fn next_speech_start(
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

#[test]
fn should_give_up_connecting_to_a_service_that_never_completes_the_handshake() {
    // Given a port that accepts connections but whose service never answers
    let (listener, url) = free_listener();
    // When
    let started = Instant::now();
    let error = voicepuncher::adapters::secondary::commandspotting::websocket::client::probe(&url)
        .unwrap_err();
    // Then
    assert!(
        started.elapsed() < BACK_WITHIN,
        "held {:?}: {error:#}",
        started.elapsed()
    );
    drop(listener);
}

/// The service ports the fake copies' addresses take: their copies land 41 000 below, on
/// 2001–8004, clear of the system's ephemeral ports (49152+) and of the user's services.
const FAKE_SERVICE_PORTS: std::ops::Range<u16> = 43_000..49_000;

/// Listeners on the first two copy ports of a command spotting service address nothing
/// listens on, and that address.
fn two_free_copies() -> (TcpListener, TcpListener, String) {
    loop {
        let base = reserve_port();
        let url = format!("ws://127.0.0.1:{base}");
        let ports = voicepuncher_copy_pool::ServiceAddress::parse(&url)
            .unwrap()
            .with_copy_offset(voicepuncher_copy_pool::COMMAND_SPOTTING_COPY_OFFSET)
            .copy_ports();
        if let (Ok(first), Ok(second)) = (
            TcpListener::bind(("127.0.0.1", ports[0])),
            TcpListener::bind(("127.0.0.1", ports[1])),
        ) {
            return (first, second, url);
        }
    }
}

/// A port of `FAKE_SERVICE_PORTS` bound without listening for the life of the test process:
/// connections to it are refused, so the client looks for copies.
fn reserve_port() -> u16 {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let span = FAKE_SERVICE_PORTS.len();
    let start = std::process::id() as usize * 7_919;
    loop {
        let index = (start + NEXT.fetch_add(1, Ordering::SeqCst)) % span;
        let port = FAKE_SERVICE_PORTS.start + index as u16;
        if bind_without_listening(port) {
            return port;
        }
    }
}

fn bind_without_listening(port: u16) -> bool {
    // SAFETY: a plain socket call with constant arguments.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    assert!(fd >= 0, "socket: {}", std::io::Error::last_os_error());
    // SAFETY: sockaddr_in is plain data, valid when zeroed.
    let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    let len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
    addr.sin_len = len as u8;
    addr.sin_family = libc::AF_INET as libc::sa_family_t;
    addr.sin_port = port.to_be();
    addr.sin_addr.s_addr = u32::from(std::net::Ipv4Addr::LOCALHOST).to_be();
    // SAFETY: `addr` is a sockaddr_in of length `len`, alive for the call; a bound descriptor
    // is never closed, so its port stays reserved until the process exits.
    let bound = unsafe { libc::bind(fd, (&raw const addr).cast(), len) } == 0;
    if !bound {
        // SAFETY: `fd` is this function's own open descriptor, closed once.
        unsafe { libc::close(fd) };
    }
    bound
}

/// A stub service on `listener` that keeps every connection, recording what it receives.
fn stub_service_that_stays(listener: TcpListener) -> Arc<Mutex<Vec<Received>>> {
    let connections = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&connections);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let connections = Arc::clone(&recorded);
            std::thread::spawn(move || serve(stream, 1, &connections));
        }
    });
    connections
}

#[test]
fn should_move_to_another_copy_at_the_first_attempt_when_the_connected_copy_goes() {
    // Given two copies: the first serves one connection and goes, the second stays
    lines();
    let (first, second, url) = two_free_copies();
    serve_one_connection_then_go(first);
    let connections = stub_service_that_stays(second);
    let (stream, audio, inputs) = start_stream(&url);
    // When
    let outage = outage_inputs(&audio, &inputs, &OnceLock::new());
    // Then
    assert_eq!(
        outage,
        [PipelineInput::SpottingLost, PipelineInput::SpottingBack],
        "{:#?}",
        logged(&url)
    );
    let failed: Vec<String> = logged(&url)
        .into_iter()
        .filter(|line| line.contains("failed"))
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
    wait_for_record(&connections, |all| {
        all.first()
            .is_some_and(|c| c.selections.iter().any(|s| s.contains("press enter")))
    });
    stream.stop();
}
