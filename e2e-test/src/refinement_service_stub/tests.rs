use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

use super::*;

const MODELS_REQUEST: &str = "GET /v1/models HTTP/1.1\r\nHost: stub\r\n\r\n";
const BAD_LENGTH_REQUEST: &str =
    "POST /v1/chat/completions HTTP/1.1\r\nContent-Length: many\r\n\r\n";

fn connect(stub: &RefinementServiceStub) -> TcpStream {
    let address = stub
        .url
        .trim_start_matches("http://")
        .trim_end_matches("/v1");
    TcpStream::connect(address).expect("the stub listens")
}

/// Sends `request` and waits until the stub closes the connection.
fn send_until_closed(stub: &RefinementServiceStub, request: &str) {
    let mut stream = connect(stub);
    stream.write_all(request.as_bytes()).expect("request sent");
    stream.shutdown(Shutdown::Write).expect("request ended");
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .expect("connection closed by the stub");
}

fn answering_stub() -> RefinementServiceStub {
    RefinementServiceStub::spawn(StubBehavior::Answer {
        content: String::new(),
        finish_reason: STOP_FINISH_REASON,
    })
    .expect("stub spawned")
}

#[test]
fn should_report_the_request_count_every_event_and_the_log() {
    let events = [StubEvent::Models, StubEvent::Probe];
    let report = missing_request_report(&events, "app log");
    let expected = "expected one request to the text refinement service, got 0; \
        the service saw 2 connections: 1. the model list was requested; \
        2. a connection was opened and closed without a request; stderr: app log";
    assert_eq!(report, expected);
}

#[test]
fn should_report_that_the_service_saw_no_connection() {
    let report = missing_request_report(&[], "");
    assert!(
        report.contains("the service saw no connection"),
        "report: {report}"
    );
}

#[test]
fn should_record_the_model_list_request() {
    let stub = answering_stub();
    send_until_closed(&stub, MODELS_REQUEST);
    assert_eq!(stub.events(), vec![StubEvent::Models]);
}

#[test]
fn should_record_a_connection_closed_without_a_byte_as_a_probe() {
    let stub = answering_stub();
    send_until_closed(&stub, "");
    assert_eq!(stub.events(), vec![StubEvent::Probe]);
}

#[test]
fn should_record_a_bad_content_length_as_dropped() {
    let stub = answering_stub();
    send_until_closed(&stub, BAD_LENGTH_REQUEST);
    let events = stub.events();
    let [StubEvent::Dropped { reason }] = events.as_slice() else {
        panic!("expected one dropped connection, got {events:?}");
    };
    assert!(reason.contains("content length"), "reason: {reason}");
}
