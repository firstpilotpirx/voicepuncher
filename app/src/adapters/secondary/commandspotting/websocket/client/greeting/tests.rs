use std::net::{TcpListener, TcpStream};

use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;

use super::*;

const REFUSAL_OF_VERSION_2: &str = "the app speaks protocol version 3, the service version 2";

/// A service on a free loopback port that answers every connection with `answer`.
fn service(answer: fn(&mut WebSocket<TcpStream>)) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://127.0.0.1:{}", listener.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut socket = tungstenite::accept(stream).unwrap();
            answer(&mut socket);
        }
    });
    url
}

fn speak_the_apps_version(socket: &mut WebSocket<TcpStream>) {
    while socket.read().is_ok() {}
}

fn refuse_as_version_2(socket: &mut WebSocket<TcpStream>) {
    let _ = socket.read();
    let _ = socket.close(Some(CloseFrame {
        code: CloseCode::Policy,
        reason: REFUSAL_OF_VERSION_2.into(),
    }));
    while socket.read().is_ok() {}
}

#[test]
fn should_accept_a_copy_that_speaks_the_apps_version() {
    // Given
    let url = service(speak_the_apps_version);
    // When
    let greeting = protocol_at(&url);
    // Then
    assert_eq!(greeting.unwrap(), Greeting::Accepted);
}

#[test]
fn should_read_the_version_of_a_copy_that_refuses_the_hello() {
    // Given
    let url = service(refuse_as_version_2);
    // When
    let greeting = protocol_at(&url);
    // Then
    assert_eq!(greeting.unwrap(), Greeting::OtherVersion(2));
}

#[test]
fn should_read_the_service_version_from_a_refusal() {
    // When
    let version = service_version(REFUSAL_OF_VERSION_2);
    // Then
    assert_eq!(version, Some(2));
}

#[test]
fn should_not_take_another_refusal_for_a_protocol_version() {
    // When
    let version = service_version("unknown phrase: open sesame");
    // Then
    assert_eq!(version, None);
}
