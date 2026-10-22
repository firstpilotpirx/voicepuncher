use std::net::{TcpListener, ToSocketAddrs};

use super::*;

/// A service on the second address `localhost` resolves to only, so the first refuses.
fn service_on_the_second_address_of_localhost() -> String {
    let addresses: Vec<_> = ("localhost", 0).to_socket_addrs().unwrap().collect();
    assert!(
        addresses.len() >= 2,
        "localhost has one address: {addresses:?}"
    );
    let listener = TcpListener::bind(addresses[1]).unwrap();
    let url = format!("ws://localhost:{}", listener.local_addr().unwrap().port());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept(stream).unwrap();
        while socket.read().is_ok() {}
    });
    url
}

#[test]
fn should_connect_through_the_second_address_when_the_first_refuses() {
    // Given
    let url = service_on_the_second_address_of_localhost();
    // When
    let connected = handshake(&url);
    // Then
    assert!(connected.is_ok(), "{:?}", connected.err());
}
