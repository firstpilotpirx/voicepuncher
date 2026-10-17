//! The real service on a loopback port, with a speech recognition service that never
//! answers: the connection must keep answering the app. Gated on the model paths the
//! justfile exports.

use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

mod common;

use voicepuncher_command_spotting::engine::{Engine, EngineCfg, SAMPLE_RATE};
use voicepuncher_command_spotting::server;

const FRAME_SAMPLES: usize = 1_600;
const READ_WAIT: Duration = Duration::from_millis(10);
/// How long connecting and the WebSocket handshake with the in-process service may take.
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(5);
/// One frame of audio, as long as it lasts in real time.
const FRAME: Duration = Duration::from_millis(100);
const PING_EVERY: Duration = Duration::from_millis(250);
/// How long the app keeps streaming after the speech, as in the incident.
const WATCH_FOR: Duration = Duration::from_secs(15);
/// The longest the app may wait for an answer to a ping.
const MAX_PONG_GAP: Duration = Duration::from_secs(1);
const SILENCE_SECS: f32 = 0.5;
/// A long utterance: the service checks it with speech recognition while it runs.
const LONG_UTTERANCE: &str =
    "so I was saying that the weather today is really nice and we could go for a walk later";
const PHRASES: &str =
    r#"{"phrases":["stop dictation","cancel dictation","restart dictation","press enter"]}"#;

/// A speech recognition service that takes every request and never answers it.
fn speech_recognition_that_never_answers() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for mut stream in listener.incoming().flatten() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            held.push(stream);
        }
    });
    port
}

fn engine_cfg(transcription_url: String) -> Option<EngineCfg> {
    let (Ok(model_dir), Ok(vad_model)) = (
        std::env::var("VOICEPUNCHER_KWS_MODEL_DIR"),
        std::env::var("VOICEPUNCHER_KWS_VAD_MODEL"),
    ) else {
        eprintln!(
            "GATED: VOICEPUNCHER_KWS_MODEL_DIR / VOICEPUNCHER_KWS_VAD_MODEL not set, skipping"
        );
        return None;
    };
    Some(EngineCfg {
        model_dir: PathBuf::from(model_dir),
        vad_model: PathBuf::from(vad_model),
        threads: 2,
        min_silence_secs: 0.3,
        save_speech_dir: None,
        keywords: None,
        transcription_url: Some(transcription_url),
        max_command_secs: 3.0,
        confirmation_deadline: Duration::from_secs(3),
    })
}

/// The service on a free port, confirming with the speech recognition service on `stt_port`.
fn start_service(stt_port: u16) -> Option<u16> {
    let cfg = engine_cfg(format!("http://127.0.0.1:{stt_port}/v1"))?;
    let engine = Arc::new(Engine::load(&cfg).expect("models load"));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || server::serve(listener, engine));
    Some(port)
}

fn connect(port: u16) -> WebSocket<MaybeTlsStream<TcpStream>> {
    let handshake_failed = "the command spotting service did not complete the handshake within 5 s";
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect_timeout(&addr, HANDSHAKE_LIMIT).expect(handshake_failed);
    stream.set_read_timeout(Some(HANDSHAKE_LIMIT)).unwrap();
    stream.set_write_timeout(Some(HANDSHAKE_LIMIT)).unwrap();
    let (mut socket, _) = tungstenite::client(
        format!("ws://127.0.0.1:{port}"),
        MaybeTlsStream::Plain(stream),
    )
    .expect(handshake_failed);
    if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
        stream.set_read_timeout(Some(READ_WAIT)).unwrap();
        stream.set_write_timeout(None).unwrap();
    }
    socket.send(Message::text(PHRASES)).unwrap();
    socket
}

fn speech_then_silence() -> Vec<f32> {
    let silence = vec![0.0f32; (SILENCE_SECS * SAMPLE_RATE as f32) as usize];
    let mut audio = silence.clone();
    audio.extend(common::spoken_by(None, LONG_UTTERANCE));
    audio.extend(silence);
    audio
}

fn send_audio(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>, samples: &[f32]) {
    for frame in samples.chunks(FRAME_SAMPLES) {
        let bytes: Vec<u8> = frame.iter().flat_map(|s| s.to_le_bytes()).collect();
        socket.send(Message::binary(bytes)).unwrap();
    }
}

/// Keeps every log line of the service, which runs in this process.
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

/// The longest time between two answers to pings sent every `PING_EVERY` while `audio`
/// streams in real time, frame after frame, for `WATCH_FOR`.
fn longest_pong_gap(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>, audio: &[f32]) -> Duration {
    let started = Instant::now();
    let (mut last_ping, mut last_answer, mut longest) = (started, started, Duration::ZERO);
    let mut last_frame = started;
    let mut frames = audio.chunks(FRAME_SAMPLES).cycle();
    socket.send(Message::Ping(Default::default())).unwrap();
    while started.elapsed() < WATCH_FOR {
        if last_frame.elapsed() >= FRAME {
            send_audio(socket, frames.next().unwrap_or_default());
            last_frame = Instant::now();
        }
        if last_ping.elapsed() >= PING_EVERY {
            socket.send(Message::Ping(Default::default())).unwrap();
            last_ping = Instant::now();
        }
        match socket.read() {
            Ok(Message::Pong(_)) => {
                longest = longest.max(last_answer.elapsed());
                last_answer = Instant::now();
            }
            Ok(Message::Close(frame)) => panic!("the service closed the connection: {frame:?}"),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("the connection ended: {e}"),
        }
    }
    longest.max(last_answer.elapsed())
}

#[test]
fn should_keep_answering_the_app_while_a_confirmation_hangs() {
    // Given
    lines();
    let Some(port) = start_service(speech_recognition_that_never_answers()) else {
        return;
    };
    let mut socket = connect(port);
    // When the app keeps streaming speech for 15 seconds
    let gap = longest_pong_gap(&mut socket, &speech_then_silence());
    // Then
    assert!(
        gap <= MAX_PONG_GAP,
        "the service went {gap:?} without answering a ping"
    );
    let given_up: Vec<String> = lines()
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|l| l.contains("given up after"))
        .cloned()
        .collect();
    assert!(
        !given_up.is_empty(),
        "no hung confirmation was logged as given up"
    );
}
