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
