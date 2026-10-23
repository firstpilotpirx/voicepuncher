//! `HttpProbe`: what a port answers, and the warm-up of a speech recognition copy, which posts
//! a short utterance to the copy's transcription path and tells whether it was transcribed.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use voicepuncher::adapters::secondary::localservices::process::HttpProbe;
use voicepuncher::core::application::{ProbeAnswer, ServiceProbe};
use voicepuncher::core::domain::ServiceKind;

/// Serves every request on a free port with `respond(path)`: (status, body).
fn server(respond: fn(&str) -> (u16, &'static str)) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = [0u8; 1024];
            let read = stream.read(&mut request).unwrap_or(0);
            let head = String::from_utf8_lossy(&request[..read]).to_string();
            let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (status, body) = respond(&path);
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    port
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn should_find_nothing_on_a_closed_port() {
    assert_eq!(
        HttpProbe::new().answer(ServiceKind::Refinement, free_port()),
        ProbeAnswer::Closed
    );
}

#[test]
fn should_take_a_healthy_service_as_ready() {
    let port = server(|_| (200, "{\"status\":\"ok\"}"));
    assert_eq!(
        HttpProbe::new().answer(ServiceKind::Refinement, port),
        ProbeAnswer::Ready
    );
}

#[test]
fn should_take_a_loading_service_as_busy() {
    let port = server(|_| (503, "{\"error\":\"Loading model\"}"));
    assert_eq!(
        HttpProbe::new().answer(ServiceKind::Refinement, port),
        ProbeAnswer::Busy
    );
}

#[test]
fn should_take_a_whisper_server_as_a_speech_recognition_service() {
    let port = server(|path| match path {
        "/health" => (200, "{\"status\":\"ok\"}"),
        _ => (200, "<title>Whisper.cpp Server</title>"),
    });
    assert_eq!(
        HttpProbe::new().answer(ServiceKind::Dictation, port),
        ProbeAnswer::Ready
    );
}

#[test]
fn should_not_take_another_healthy_program_as_a_speech_recognition_service() {
    let port = server(|_| (200, "{\"status\":\"ok\"}"));
    assert_eq!(
        HttpProbe::new().answer(ServiceKind::Commands, port),
        ProbeAnswer::Busy
    );
}

/// What the stub received: the request line and how many body bytes followed it.
struct Received {
    line: String,
    body_bytes: usize,
}

/// Reads one request whole; its request line and how many body bytes it had.
fn read_request(reader: &mut impl BufRead) -> (String, usize) {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let mut length = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        if header.trim().is_empty() {
            break;
        }
        if let Some(value) = header.to_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).unwrap();
    (line, length)
}

/// Answers every request on a free port with `status`, after reading it whole; reports each one.
fn stub(status: u16) -> (u16, Receiver<Received>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sent, received) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (line, length) = read_request(&mut BufReader::new(stream.try_clone().unwrap()));
            let answer = "{\"text\":\" Hello there.\"}";
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            );
            let _ = sent.send(Received { line: line.trim().to_string(), body_bytes: length });
        }
    });
    (port, received)
}

#[test]
fn should_post_the_warm_up_utterance_to_the_transcription_path_of_a_speech_recognition_copy() {
    // Given
    let (port, received) = stub(200);
    // When
    let warmed = HttpProbe::new().warm_up(ServiceKind::Commands, port);
    // Then
    assert!(warmed.is_some_and(|took| !took.is_zero()), "{warmed:?}");
    let request = received.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(request.line, "POST /v1/audio/transcriptions HTTP/1.1");
    assert!(
        request.body_bytes > 16_000,
        "{} body bytes",
        request.body_bytes
    );
}

#[test]
fn should_not_count_a_copy_warm_when_it_fails_the_transcription() {
    // Given
    let (port, _received) = stub(500);
    // When
    let warmed = HttpProbe::new().warm_up(ServiceKind::Dictation, port);
    // Then
    assert_eq!(warmed, None);
}

#[test]
fn should_send_nothing_to_a_service_that_transcribes_nothing() {
    // Given
    let (port, received) = stub(200);
    // When
    let warmed = HttpProbe::new().warm_up(ServiceKind::Refinement, port);
    // Then
    assert_eq!(warmed, Some(Duration::ZERO));
    assert!(received.recv_timeout(Duration::from_millis(300)).is_err());
}
