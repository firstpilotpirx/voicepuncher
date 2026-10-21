//! The command spotting client's wait for the service against stub services on loopback:
//! one that answers pings at once but its events late, one that answers pings but never
//! the sync, and one that stops answering; and a pause of the client's own against a service
//! that answers pings.

use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use tungstenite::{Message, WebSocket};

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;

/// How late the stub service answers each audio message: more than the client's silence
/// limit (5 s), which counts only a service that answers nothing at all.
const LATE_EVENTS: Duration = Duration::from_secs(6);
const QUICK_EVENTS: Duration = Duration::from_millis(200);
const FRAMES: usize = 3;
/// 100 ms of 16 kHz audio, as the app sends it.
const FRAME_SAMPLES: usize = 1_600;
const READ_WAIT: Duration = Duration::from_millis(10);
/// The client's silence limit (5 s), less a margin: it must not give up before that.
const PATIENT_FOR_AT_LEAST: Duration = Duration::from_millis(4_500);
/// The silence limit plus a margin: a silent service is noticed within it.
const HANG_NOTICED_WITHIN: Duration = Duration::from_secs(7);
/// A pause of the client's own, as when it is busy or the computer sleeps: longer than its
/// silence limit.
const CLIENT_PAUSE: Duration = Duration::from_secs(6);
/// Longer than the client's ping interval (1 s): a ping is out once it checks again.
const PING_DUE: Duration = Duration::from_millis(1_100);
/// How long the client waits for the answer to a sync from a service that sends it, at
/// most 12 s late: the bound the E2E tests give the real service.
const SYNC_ANSWER_LIMIT: Duration = Duration::from_secs(20);
/// How long the client waits for the answer to a sync from a service that never sends it.
const SYNC_BOUND: Duration = Duration::from_secs(2);
/// The sync bound plus a margin: the client gives up within it.
const SYNC_GIVEN_UP_WITHIN: Duration = Duration::from_secs(4);

/// A service that answers pings at once and each audio message with a speech start `delay`
/// after it came, and a sync once every earlier audio message is answered.
fn service_answering_late(delay: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let socket = tungstenite::accept(stream).unwrap();
        socket.get_ref().set_read_timeout(Some(READ_WAIT)).unwrap();
        answer_late(socket, delay);
    });
    url
}

/// Due answers in the order their messages came: events, or the answer to a sync.
fn answer_late(mut socket: WebSocket<TcpStream>, delay: Duration) {
    let mut due: Vec<(Instant, String)> = Vec::new();
    let mut answered = 0u32;
    loop {
        match socket.read() {
            Ok(Message::Binary(_)) => {
                answered += 1;
                due.push((
                    Instant::now() + delay,
                    format!(r#"{{"event":"speech_start","t":{answered}}}"#),
                ));
            }
            Ok(Message::Text(text)) => {
                let sync: serde_json::Value = serde_json::from_str(text.as_str()).unwrap();
                let at = due.last().map_or_else(Instant::now, |(at, _)| *at);
                due.push((at, format!(r#"{{"event":"synced","id":{}}}"#, sync["sync"])));
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return,
        }
        while due.first().is_some_and(|(at, _)| *at <= Instant::now()) {
            let (_, json) = due.remove(0);
            if socket.send(Message::text(json)).is_err() {
                return;
            }
        }
    }
}

/// A service that answers pings, and reads everything else without ever answering it: the
/// sync is never answered.
fn service_never_answering_the_sync() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        socket.get_ref().set_read_timeout(Some(READ_WAIT)).unwrap();
        loop {
            match socket.read() {
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return,
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

fn client_with_audio_sent(url: &str) -> SpottingClient {
    let mut client = SpottingClient::connect(url, Instant::now()).unwrap();
    for _ in 0..FRAMES {
        client.send_audio(&[0.0; FRAME_SAMPLES]).unwrap();
    }
    client
}

#[test]
fn should_return_every_event_for_the_audio_sent_once_the_service_answered_it() {
    // Given
    let mut client = client_with_audio_sent(&service_answering_late(QUICK_EVENTS));
    // When
    let events = client.events_for_sent_audio(SYNC_ANSWER_LIMIT).unwrap();
    // Then
    assert_eq!(events.len(), FRAMES, "{events:?}");
}

#[test]
fn should_wait_for_a_service_that_answers_pings_when_its_events_come_late_within_the_bound() {
    // Given
    let mut client = client_with_audio_sent(&service_answering_late(LATE_EVENTS));
    // When
    let started = Instant::now();
    let events = client.events_for_sent_audio(SYNC_ANSWER_LIMIT).unwrap();
    // Then
    assert_eq!(events.len(), FRAMES, "{events:?}");
    assert!(started.elapsed() >= LATE_EVENTS);
}

#[test]
fn should_fail_the_wait_when_the_service_stops_answering() {
    // Given
    let url = hanging_service();
    let mut client = SpottingClient::connect(&url, Instant::now()).unwrap();
    client.send_audio(&[0.0; FRAME_SAMPLES]).unwrap();
    // When
    let started = Instant::now();
    let error = format!(
        "{:#}",
        client.events_for_sent_audio(SYNC_ANSWER_LIMIT).unwrap_err()
    );
    // Then
    let waited = started.elapsed();
    assert!(
        waited >= PATIENT_FOR_AT_LEAST,
        "gave up after {waited:?}: {error}"
    );
    assert!(waited < HANG_NOTICED_WITHIN, "{error}");
    assert!(
        error.contains("command spotting service stopped answering"),
        "{error}"
    );
}

#[test]
fn should_fail_the_wait_within_its_bound_when_the_service_answers_pings_but_never_the_sync() {
    // Given
    let mut client = client_with_audio_sent(&service_never_answering_the_sync());
    // When
    let started = Instant::now();
    let error = format!(
        "{:#}",
        client.events_for_sent_audio(SYNC_BOUND).unwrap_err()
    );
    // Then
    let waited = started.elapsed();
    assert!(waited >= SYNC_BOUND, "gave up after {waited:?}: {error}");
    assert!(waited < SYNC_GIVEN_UP_WITHIN, "{error}");
    assert!(error.contains("command spotting service"), "{error}");
    assert!(error.contains("sync 1"), "{error}");
}

/// The client's round as the streaming worker runs it: check that the service answers, then
/// take its events.
fn exchange(client: &mut SpottingClient) -> anyhow::Result<()> {
    client.keep_alive()?;
    client.read_events()?;
    Ok(())
}

#[test]
fn should_not_count_a_pause_of_its_own_with_a_ping_out_as_the_service_going_away() {
    // Given a service that answers pings, and a ping out
    let mut client =
        SpottingClient::connect(&service_never_answering_the_sync(), Instant::now()).unwrap();
    std::thread::sleep(PING_DUE);
    exchange(&mut client).unwrap();
    // When the client reads nothing for longer than its silence limit
    std::thread::sleep(CLIENT_PAUSE);
    // Then the service is not lost: the client logs no error of its own and leaves the error
    // to whoever gets its failure, so an answered exchange also means no error is logged
    exchange(&mut client).unwrap();
}

#[test]
fn should_not_count_a_pause_of_its_own_right_after_connecting_as_the_service_going_away() {
    // Given a service that answers pings, and no ping out yet
    let mut client =
        SpottingClient::connect(&service_never_answering_the_sync(), Instant::now()).unwrap();
    // When the client reads nothing for longer than its silence limit
    std::thread::sleep(CLIENT_PAUSE);
    // Then
    exchange(&mut client).unwrap();
}
