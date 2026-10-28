//! `HttpProbe`: the protocol version a copy of the command spotting service speaks, from its
//! answer to the app's hello.

use std::net::{TcpListener, TcpStream};

use tungstenite::WebSocket;
use voicepuncher::adapters::secondary::commandspotting::websocket::test_stream::refuse_as_version_2;
use voicepuncher::adapters::secondary::localservices::process::HttpProbe;
use voicepuncher::core::application::{OtherProtocol, ServiceProbe};
use voicepuncher::core::domain::ServiceKind;

/// A command spotting copy on a free port that answers every connection with `answer`.
fn copy(answer: fn(&mut WebSocket<TcpStream>)) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if let Ok(mut socket) = tungstenite::accept(stream) {
                answer(&mut socket);
            }
        }
    });
    port
}

fn takes_the_hello(socket: &mut WebSocket<TcpStream>) {
    while socket.read().is_ok() {}
}

#[test]
fn should_name_the_version_of_a_copy_that_refuses_the_hello() {
    // Given
    let port = copy(refuse_as_version_2);
    // When
    let other = HttpProbe::new().other_protocol(ServiceKind::CommandSpotting, port);
    // Then
    assert_eq!(other, Some(OtherProtocol { service: 2, app: 3 }));
}

#[test]
fn should_name_no_other_version_for_a_copy_that_takes_the_hello() {
    // Given
    let port = copy(takes_the_hello);
    // When
    let other = HttpProbe::new().other_protocol(ServiceKind::CommandSpotting, port);
    // Then
    assert_eq!(other, None);
}
