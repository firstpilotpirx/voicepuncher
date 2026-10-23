//! The real service on a loopback port, with a speech recognition service that never
//! answers: the connection must keep answering the app. Needs the models the justfile finds
//! (`needs_kws_models`).

use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::common;
use crate::kws_models::kws_models;

use voicepuncher_command_spotting::engine::{Engine, EngineCfg, SAMPLE_RATE};
use voicepuncher_command_spotting::server;

const FRAME_SAMPLES: usize = 1_600;
const READ_WAIT: Duration = Duration::from_millis(10);
/// How long connecting and the WebSocket handshake with the in-process service may take.
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(5);
/// One frame of audio, as long as it lasts in real time.
const FRAME: Duration = Duration::from_millis(100);
const PING_EVERY: Duration = Duration::from_millis(250);
/// How long the service waits for a confirmation: short, so the test streams past it in seconds
/// instead of the default 3 s.
const CONFIRMATION_DEADLINE: Duration = Duration::from_secs(1);
/// How long the app keeps streaming past the speech: past the wait for its confirmation and the
/// second attempt of the whole utterance.
const WATCH_PAST_SPEECH: Duration = Duration::from_secs(2);
/// The longest the app may wait for an answer to a ping.
const MAX_PONG_GAP: Duration = Duration::from_secs(1);
const SILENCE_SECS: f32 = 0.5;
/// A long utterance: the service checks it with speech recognition while it runs.
const LONG_UTTERANCE: &str =
    "so I was saying that the weather today is really nice and we could go for a walk later";
const HELLO: &str = r#"{"type":"hello","version":2}"#;
const PHRASES: &str = r#"{"type":"select","phrases":["stop dictation","cancel dictation","restart dictation","press enter"]}"#;
/// How long the service may take to close a connection it refuses.
const CLOSE_LIMIT: Duration = Duration::from_secs(5);

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

fn engine_cfg(transcription_url: String) -> EngineCfg {
    let (model_dir, vad_model) = kws_models();
    EngineCfg {
        model_dir,
        vad_model,
        threads: 2,
        min_silence_secs: 0.3,
        save_speech_dir: None,
        keywords: None,
        transcription_url: Some(transcription_url),
        max_command_secs: 3.0,
        confirmation_deadline: CONFIRMATION_DEADLINE,
    }
}

/// The service on a free port, confirming with the speech recognition service on `stt_port`.
fn start_service(stt_port: u16) -> u16 {
    let cfg = engine_cfg(format!("http://127.0.0.1:{stt_port}/v1"));
    let engine = Arc::new(Engine::load(&cfg).expect("models load"));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || server::serve(listener, engine));
    port
}

fn connect(port: u16) -> WebSocket<MaybeTlsStream<TcpStream>> {
    let mut socket = handshake(port);
    socket.send(Message::text(HELLO)).unwrap();
    socket.send(Message::text(PHRASES)).unwrap();
    socket
}

/// A WebSocket to the service that has sent nothing yet.
fn handshake(port: u16) -> WebSocket<MaybeTlsStream<TcpStream>> {
    let handshake_failed = "the command spotting service did not complete the handshake within 5 s";
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect_timeout(&addr, HANDSHAKE_LIMIT).expect(handshake_failed);
    stream.set_read_timeout(Some(HANDSHAKE_LIMIT)).unwrap();
    stream.set_write_timeout(Some(HANDSHAKE_LIMIT)).unwrap();
    let (socket, _) = tungstenite::client(
        format!("ws://127.0.0.1:{port}"),
        MaybeTlsStream::Plain(stream),
    )
    .expect(handshake_failed);
    if let MaybeTlsStream::Plain(stream) = socket.get_ref() {
        stream.set_read_timeout(Some(READ_WAIT)).unwrap();
        stream.set_write_timeout(None).unwrap();
    }
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
/// streams in real time, frame after frame, for `watch_for`.
fn longest_pong_gap(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    audio: &[f32],
    watch_for: Duration,
) -> Duration {
    let started = Instant::now();
    let (mut last_ping, mut last_answer, mut longest) = (started, started, Duration::ZERO);
    let mut last_frame = started;
    let mut frames = audio.chunks(FRAME_SAMPLES).cycle();
    socket.send(Message::Ping(Default::default())).unwrap();
    while started.elapsed() < watch_for {
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

/// The code and reason the service closes the connection with, once `sent` went out.
fn closed_after(sent: &[Message]) -> (CloseCode, String) {
    let port = start_service(speech_recognition_that_never_answers());
    let mut socket = handshake(port);
    for message in sent {
        socket.send(message.clone()).unwrap();
    }
    let deadline = Instant::now() + CLOSE_LIMIT;
    while Instant::now() < deadline {
        match socket.read() {
            Ok(Message::Close(Some(frame))) => return (frame.code, frame.reason.to_string()),
            Ok(Message::Close(None)) => {
                panic!("the service closed the connection without a reason")
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("the connection ended without a close: {e}"),
        }
    }
    panic!("the service did not close the connection within {CLOSE_LIMIT:?}")
}

mod needs_kws_models {
    use super::*;

    #[test]
    fn should_keep_answering_the_app_while_a_confirmation_hangs() {
        // Given
        lines();
        let port = start_service(speech_recognition_that_never_answers());
        let mut socket = connect(port);
        let audio = speech_then_silence();
        let speech = Duration::from_secs_f32(audio.len() as f32 / SAMPLE_RATE as f32);
        // When the app keeps streaming past the wait for the confirmation
        let gap = longest_pong_gap(&mut socket, &audio, speech + WATCH_PAST_SPEECH);
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

    #[test]
    fn should_refuse_an_app_of_another_protocol_version() {
        // When
        let (code, reason) = closed_after(&[Message::text(r#"{"type":"hello","version":1}"#)]);
        // Then
        assert_eq!(code, CloseCode::Policy);
        assert!(
            reason.contains("version 1") && reason.contains("version 2"),
            "{reason}"
        );
    }

    #[test]
    fn should_refuse_an_app_that_does_not_say_its_protocol_version_first() {
        // When
        let (code, reason) = closed_after(&[Message::text(r#"{"phrases":["press enter"]}"#)]);
        // Then
        assert_eq!(code, CloseCode::Policy);
        assert!(
            reason.contains("version 1") && reason.contains("version 2"),
            "{reason}"
        );
    }

    #[test]
    fn should_close_a_malformed_message_with_its_reason() {
        // When
        let (code, reason) = closed_after(&[Message::text(HELLO), Message::text("{nonsense")]);
        // Then
        assert_eq!(code, CloseCode::Invalid);
        assert!(reason.contains("malformed message"), "{reason}");
    }

    #[test]
    fn should_close_an_audio_frame_of_partial_samples_with_its_reason() {
        // When
        let partial = Message::binary(vec![0u8; 5]);
        let (code, reason) = closed_after(&[Message::text(HELLO), partial]);
        // Then
        assert_eq!(code, CloseCode::Invalid);
        assert!(reason.contains("5 bytes"), "{reason}");
    }
}
