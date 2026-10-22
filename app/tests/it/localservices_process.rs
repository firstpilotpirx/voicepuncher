use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use voicepuncher::adapters::secondary::localservices::process::{HttpProbe, ProcessLauncher};
use voicepuncher::core::application::{ProbeAnswer, ServiceLauncher, ServiceProbe, ServiceProcess};
use voicepuncher::core::domain::{ServiceKind, ServiceSpec};

fn shell(script: &str, log_file: &Path) -> ServiceSpec {
    ServiceSpec {
        kind: ServiceKind::Refinement,
        label: "test service".into(),
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), script.into()],
        port: 1,
        log_file: log_file.to_path_buf(),
        health_checked: true,
        problem: None,
    }
}

fn ended(process: &mut dyn ServiceProcess, within: Duration) -> Option<String> {
    let deadline = Instant::now() + within;
    while Instant::now() < deadline {
        if let Some(how) = process.exited() {
            return Some(how);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

fn read_when(path: &Path, wanted: &[&str]) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if wanted.iter().all(|line| text.contains(line)) || Instant::now() > deadline {
            return text;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn should_write_both_output_streams_of_a_service_into_its_log_file() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let log_file = home.path().join("logs/engine-1.log");
    // When
    let mut process = ProcessLauncher
        .spawn(&shell(
            "echo out-line; echo err-line >&2; sleep 30",
            &log_file,
        ))
        .unwrap();
    let text = read_when(&log_file, &["out-line", "err-line"]);
    process.kill();
    // Then
    assert!(text.contains("out-line"), "{text}");
    assert!(text.contains("err-line"), "{text}");
}

#[test]
fn should_end_a_service_asked_to_end() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let mut process = ProcessLauncher
        .spawn(&shell("exec sleep 30", &home.path().join("a.log")))
        .unwrap();
    // When
    process.terminate();
    // Then
    assert_eq!(
        ended(process.as_mut(), Duration::from_secs(5)).as_deref(),
        Some("signal 15")
    );
}

#[test]
fn should_kill_a_service_that_ignores_the_request_to_end() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let mut process = ProcessLauncher
        .spawn(&shell(
            "trap '' TERM; while true; do sleep 1; done",
            &home.path().join("a.log"),
        ))
        .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    process.terminate();
    let after_terminate = ended(process.as_mut(), Duration::from_millis(500));
    // When
    process.kill();
    // Then
    assert_eq!(after_terminate, None);
    assert_eq!(process.exited().as_deref(), Some("signal 9"));
}

#[test]
fn should_tell_the_exit_code_of_a_service_that_exits() {
    // Given
    let home = tempfile::tempdir().unwrap();
    // When
    let mut process = ProcessLauncher
        .spawn(&shell("exit 3", &home.path().join("a.log")))
        .unwrap();
    // Then
    assert_eq!(
        ended(process.as_mut(), Duration::from_secs(5)).as_deref(),
        Some("code 3")
    );
}

#[test]
fn should_refuse_to_start_a_missing_program() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let mut spec = shell("", &home.path().join("a.log"));
    spec.program = home.path().join("no-such-engine");
    // When
    let started = ProcessLauncher.spawn(&spec);
    // Then
    assert!(started.is_err());
}

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
