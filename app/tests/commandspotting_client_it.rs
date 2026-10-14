//! The command spotting client's wait for the service against stub services on loopback:
//! one that answers every audio message late, and one that stops answering.

use std::net::TcpListener;
use std::time::{Duration, Instant};

use tungstenite::Message;

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;

/// How late the stub service answers each audio message.
const ANSWER_DELAY: Duration = Duration::from_millis(200);
const FRAMES: usize = 3;
/// 100 ms of 16 kHz audio, as the app sends it.
const FRAME_SAMPLES: usize = 1_600;
/// The client's silence limit (10 s), less a margin: it must not give up on a slow service
/// before that.
const PATIENT_FOR_AT_LEAST: Duration = Duration::from_secs(9);
/// Longer than the client waits for a silent service before it gives up on it.
const HANG_NOTICED_WITHIN: Duration = Duration::from_secs(15);

/// A service that answers each audio message with a speech start, `delay` late, and
/// handles messages one at a time, as the real service does.
fn service_answering_late(delay: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        let mut answered = 0u32;
        while let Ok(message) = socket.read() {
            if message.is_binary() {
                std::thread::sleep(delay);
                answered += 1;
                let event = format!(r#"{{"event":"speech_start","t":{answered}}}"#);
                if socket.send(Message::text(event)).is_err() {
                    return;
                }
            }
        }
    });
    url
}

/// A service that accepts the connection, then hangs: it never reads or answers again.
fn hanging_service() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let socket = tungstenite::accept(stream).unwrap();
        std::thread::sleep(Duration::from_secs(600));
        drop(socket);
    });
    url
}

#[test]
fn should_return_every_event_for_the_audio_sent_once_the_service_answered_it() {
    // Given
    let url = service_answering_late(ANSWER_DELAY);
    let mut client = SpottingClient::connect(&url, Instant::now()).unwrap();
    for _ in 0..FRAMES {
        client.send_audio(&[0.0; FRAME_SAMPLES]).unwrap();
    }
    // When
    let events = client.events_for_sent_audio().unwrap();
    // Then
    assert_eq!(events.len(), FRAMES, "{events:?}");
}

#[test]
fn should_fail_the_wait_when_the_service_stops_answering() {
    // Given
    let url = hanging_service();
    let mut client = SpottingClient::connect(&url, Instant::now()).unwrap();
    client.send_audio(&[0.0; FRAME_SAMPLES]).unwrap();
    // When
    let started = Instant::now();
    let error = format!("{:#}", client.events_for_sent_audio().unwrap_err());
    // Then
    let waited = started.elapsed();
    assert!(
        waited >= PATIENT_FOR_AT_LEAST,
        "gave up after {waited:?}: {error}"
    );
    assert!(waited < HANG_NOTICED_WITHIN, "{error}");
    assert!(error.contains("stopped answering"), "{error}");
}
