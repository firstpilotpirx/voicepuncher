use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use voicepuncher::adapters::secondary::localservices::process::ProcessLauncher;
use voicepuncher::core::application::{ServiceLauncher, ServiceProcess};
use voicepuncher::core::domain::{ServiceKind, ServiceSpec};

fn shell(script: &str, log_file: &Path) -> ServiceSpec {
    ServiceSpec {
        kind: ServiceKind::CommandSpotting,
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
fn should_have_a_service_write_both_output_streams_into_its_log_file_itself() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let log_file = home.path().join("logs/engine-1.log");
    // When
    let mut process = ProcessLauncher
        .spawn(&shell(
            "o=file; [ -p /dev/stdout ] && o=pipe; e=file; [ -p /dev/stderr ] && e=pipe; echo out-$o$o; echo err-$e$e >&2; sleep 30",
            &log_file,
        ))
        .unwrap();
    let text = read_when(&log_file, &["out-filefile", "err-filefile"]);
    process.kill();
    // Then — written into the file itself, not through a pipe of the app, so it outlives the app
    assert!(text.contains("out-filefile"), "{text}");
    assert!(text.contains("err-filefile"), "{text}");
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

mod holder;
mod probe;
mod protocol;
