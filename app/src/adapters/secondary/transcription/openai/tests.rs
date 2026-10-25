use super::*;
use std::net::TcpListener;

fn cfg_for_port(port: u16) -> TranscriptionCfg {
    TranscriptionCfg {
        base_url: format!("http://127.0.0.1:{port}/v1"),
        ..TranscriptionCfg::default()
    }
}

#[test]
fn should_connect_when_service_port_is_open() {
    // Given
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    // When
    let transcriber = OpenAiTranscriber::connect(cfg_for_port(port)).unwrap();
    // Then
    let ports: Vec<u16> = transcriber.pool.snapshot().iter().map(|c| c.port).collect();
    assert_eq!(ports, [port]);
}

#[test]
fn should_report_address_and_start_command_when_port_is_closed() {
    // Given
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    // When
    let message = OpenAiTranscriber::connect(cfg_for_port(port))
        .err()
        .unwrap()
        .to_string();
    // Then
    assert!(
        message.contains(&format!("127.0.0.1:{port}/v1")),
        "{message}"
    );
    assert!(message.contains("just start-model-servers"), "{message}");
}
