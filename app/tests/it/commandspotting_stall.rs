//! The command spotting adapter's client leaves a service that says it stalled or dropped
//! audio, so the adapter's outage path connects again and replays the audio.

use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use tungstenite::Message;

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::adapters::secondary::commandspotting::websocket::AudioStream;
use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::VoiceCommand;

const READ_WAIT: Duration = Duration::from_millis(10);
/// How long the client may take to read the service's message and fail.
const NOTICED_WITHIN: Duration = Duration::from_secs(2);
/// The adapter reports the loss and is back on a new connection well within this.
const BACK_WITHIN: Duration = Duration::from_secs(5);
const MIC_RATE_HZ: u32 = 48_000;

/// A service that sends `event` as soon as the app connects, then answers pings only.
fn service_sending(event: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        socket.get_ref().set_read_timeout(Some(READ_WAIT)).unwrap();
        socket.send(Message::text(event)).unwrap();
        while socket.read().is_ok() || socket.can_read() {}
    });
    url
}

/// The client's error once it read the service's message; none within the wait.
fn error_read_from(url: &str) -> Option<String> {
    let mut client = SpottingClient::connect(url, Instant::now()).unwrap();
    let deadline = Instant::now() + NOTICED_WITHIN;
    while Instant::now() < deadline {
        if let Err(e) = client.read_events() {
            return Some(format!("{e:#}"));
        }
    }
    None
}

#[test]
fn should_leave_a_service_that_says_a_confirmation_stalled() {
    // Given
    let url = service_sending(r#"{"event":"stalled","waited_ms":12000}"#);
    // When
    let error = error_read_from(&url);
    // Then
    let error = error.expect("the client kept the stalled service");
    assert!(error.contains("stalled"), "{error}");
}

#[test]
fn should_leave_a_service_that_dropped_audio() {
    // Given
    let url = service_sending(r#"{"event":"dropped","samples":1600}"#);
    // When
    let error = error_read_from(&url);
    // Then
    let error = error.expect("the client kept the service that dropped audio");
    assert!(error.contains("dropped 1600 samples"), "{error}");
}

/// A service that sends `event` on its first connection only and keeps every later one;
/// the count of connections it accepted.
fn service_sending_once(event: &'static str) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&accepted);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let first = count.fetch_add(1, Ordering::SeqCst) == 0;
            std::thread::spawn(move || serve(stream, first.then_some(event)));
        }
    });
    (url, accepted)
}

/// Sends `event` when there is one, then reads until the app leaves.
fn serve(stream: std::net::TcpStream, event: Option<&str>) {
    let mut socket = tungstenite::accept(stream).unwrap();
    socket.get_ref().set_read_timeout(Some(READ_WAIT)).unwrap();
    if let Some(event) = event {
        socket.send(Message::text(event)).unwrap();
    }
    while socket.read().is_ok() || socket.can_read() {}
}

/// The inputs the adapter sent while it was fed silence, until `last` or `BACK_WITHIN`.
fn inputs_of_the_adapter_until(url: &str, last: &PipelineInput) -> Vec<PipelineInput> {
    let (audio, audio_rx) = mpsc::channel();
    let (inputs_tx, inputs) = mpsc::channel();
    let stream = AudioStream::start(
        url,
        &VoiceCommand::STANDBY,
        Instant::now(),
        MIC_RATE_HZ,
        audio_rx,
        inputs_tx,
    )
    .unwrap();
    let deadline = Instant::now() + BACK_WITHIN;
    let mut got = Vec::new();
    while Instant::now() < deadline && got.last() != Some(last) {
        audio.send(vec![0.0; 4_800]).unwrap();
        got.extend(inputs.recv_timeout(Duration::from_millis(100)));
    }
    stream.stop();
    got
}

#[test]
fn should_reach_for_a_service_again_after_it_stalled_or_dropped_audio() {
    for event in [
        r#"{"event":"stalled","waited_ms":12000}"#,
        r#"{"event":"dropped","samples":1600}"#,
    ] {
        // Given
        let (url, accepted) = service_sending_once(event);
        // When
        let inputs = inputs_of_the_adapter_until(&url, &PipelineInput::SpottingBack);
        // Then
        assert!(
            inputs.contains(&PipelineInput::SpottingLost),
            "{event}: {inputs:?}"
        );
        assert_eq!(
            inputs.last(),
            Some(&PipelineInput::SpottingBack),
            "{event}: {inputs:?}"
        );
        assert!(
            accepted.load(Ordering::SeqCst) >= 2,
            "{event}: no second connection"
        );
    }
}
