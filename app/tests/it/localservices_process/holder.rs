//! The program on a service's port: stopped when it is the service's own program, left running
//! when it is another one.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use voicepuncher::adapters::secondary::localservices::process::ProcessLauncher;
use voicepuncher::core::application::{HolderStop, ServiceLauncher};
use voicepuncher::core::domain::{ServiceKind, ServiceSpec};
use voicepuncher_test_steps::loopback::SilentListener;

/// The service's program, as the catalog names it.
const PROGRAM: &str = "/opt/engines/whisper-server-test";

fn spec_on(port: u16) -> ServiceSpec {
    ServiceSpec {
        kind: ServiceKind::Commands,
        label: "commands copy".into(),
        program: PathBuf::from(PROGRAM),
        args: Vec::new(),
        port,
        log_file: PathBuf::from("/tmp/unused.log"),
        health_checked: true,
        problem: None,
    }
}

/// Whether the listener ends within 5 s.
fn ends(listener: &mut SilentListener) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while listener.runs() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    !listener.runs()
}

#[test]
fn should_stop_the_holder_whose_command_line_names_the_service_program() {
    // Given
    let mut hung = SilentListener::start(PROGRAM);
    // When
    let stop = ProcessLauncher.stop_holder(&spec_on(hung.port));
    // Then
    assert_eq!(stop, HolderStop::Stopped(hung.pid()));
    assert!(ends(&mut hung), "the hung copy still runs");
}

#[test]
fn should_leave_another_program_on_the_port_running_and_name_it() {
    // Given
    let mut other = SilentListener::start("/usr/local/bin/someone-elses-server");
    // When
    let stop = ProcessLauncher.stop_holder(&spec_on(other.port));
    // Then
    let HolderStop::Foreign(named) = stop else {
        panic!("the other program was taken for the service: {stop:?}");
    };
    assert!(named.starts_with("perl"), "{named}");
    assert!(
        named.ends_with(&format!(" (pid {})", other.pid())),
        "{named}"
    );
    assert!(other.runs(), "the other program was stopped");
}
