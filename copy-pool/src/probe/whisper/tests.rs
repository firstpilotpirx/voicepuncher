use std::net::TcpListener;
use std::thread;

use super::*;

/// A program on a loopback port that answers every connection with `answer`, or never when
/// `answer` is `None`; its port.
fn program(answer: Option<&'static [u8]>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            match answer {
                Some(answer) => drop(stream.write_all(answer)),
                None => thread::sleep(Duration::from_secs(5)),
            }
        }
    });
    port
}

const WHISPER_PAGE: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Length: 30\r\n\r\n<title>Whisper.cpp Server</title>";

#[test]
fn should_take_a_whisper_server_for_the_service() {
    let port = program(Some(WHISPER_PAGE));
    assert_eq!(
        WhisperProbe::new("127.0.0.1").identify(port),
        Identity::Service
    );
}

#[test]
fn should_name_a_program_that_speaks_http2_only_as_foreign() {
    let port = program(Some(&[0, 0, 6, 4, 0, 0, 0, 0, 0, 0, 5, 0, 0, 64, 0]));
    let Identity::Foreign(foreign) = WhisperProbe::new("127.0.0.1").identify(port) else {
        panic!("an HTTP/2 program taken for the service");
    };
    assert_eq!(
        (foreign.port, foreign.pid),
        (port, Some(std::process::id()))
    );
    assert!(foreign.name.is_some(), "{foreign:?}");
}

#[test]
fn should_take_another_http_server_as_foreign() {
    let port = program(Some(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"));
    assert!(matches!(
        WhisperProbe::new("127.0.0.1").identify(port),
        Identity::Foreign(_)
    ));
}

#[test]
fn should_not_tell_a_program_that_does_not_answer_in_time() {
    let port = program(None);
    assert_eq!(
        WhisperProbe::new("127.0.0.1").identify(port),
        Identity::Unknown
    );
}
