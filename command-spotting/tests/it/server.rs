//! The real service on a loopback port: an app that breaks the protocol is refused with a
//! reason. Needs the streaming models the justfile finds (`needs_streaming_models`).

use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use voicepuncher_command_spotting::listening::Service;
use voicepuncher_command_spotting::protocol::PROTOCOL_VERSION;
use voicepuncher_command_spotting::server;

use crate::streaming::load_engine;

const READ_WAIT: Duration = Duration::from_millis(10);
/// How long connecting and the WebSocket handshake with the in-process service may take.
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(5);
/// How long the service may take to close a connection it refuses.
const CLOSE_LIMIT: Duration = Duration::from_secs(5);

/// The port of the one service of this process: the models load once.
fn service_port() -> u16 {
    static PORT: OnceLock<u16> = OnceLock::new();
    *PORT.get_or_init(|| {
        let service = Arc::new(Service::Streaming(load_engine()));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || server::serve(&listener, &service));
        port
    })
}

fn hello() -> String {
    format!(r#"{{"type":"hello","version":{PROTOCOL_VERSION}}}"#)
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

/// The code and reason the service closes the connection with, once `sent` went out.
fn closed_after(sent: &[Message]) -> (CloseCode, String) {
    let mut socket = handshake(service_port());
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

mod needs_streaming_models {
    use super::*;

    #[test]
    fn should_refuse_an_app_of_another_protocol_version() {
        // When
        let (code, reason) = closed_after(&[Message::text(r#"{"type":"hello","version":1}"#)]);
        // Then
        assert_eq!(code, CloseCode::Policy);
        let speaks = format!("version {PROTOCOL_VERSION}");
        assert!(
            reason.contains("version 1") && reason.contains(&speaks),
            "{reason}"
        );
    }

    #[test]
    fn should_refuse_an_app_that_does_not_say_its_protocol_version_first() {
        // When
        let (code, reason) = closed_after(&[Message::text(r#"{"phrases":["press enter"]}"#)]);
        // Then
        assert_eq!(code, CloseCode::Policy);
        let speaks = format!("version {PROTOCOL_VERSION}");
        assert!(
            reason.contains("version 1") && reason.contains(&speaks),
            "{reason}"
        );
    }

    #[test]
    fn should_close_a_malformed_message_with_its_reason() {
        // When
        let (code, reason) = closed_after(&[Message::text(hello()), Message::text("{nonsense")]);
        // Then
        assert_eq!(code, CloseCode::Invalid);
        assert!(reason.contains("malformed message"), "{reason}");
    }

    #[test]
    fn should_close_an_audio_frame_of_partial_samples_with_its_reason() {
        // When
        let partial = Message::binary(vec![0u8; 5]);
        let (code, reason) = closed_after(&[Message::text(hello()), partial]);
        // Then
        assert_eq!(code, CloseCode::Invalid);
        assert!(reason.contains("5 bytes"), "{reason}");
    }
}
