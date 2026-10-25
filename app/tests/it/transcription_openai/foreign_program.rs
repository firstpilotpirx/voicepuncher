//! Another program, no speech recognition service, on a port of the service.

use super::*;

/// A program that speaks HTTP/2 only, as the `go run` server that held 8081 did.
pub(super) use voicepuncher_test_steps::loopback::Http2Only as ForeignProgram;

/// A listener on a service port of `FAKE_SERVICE_PORTS` and on its first `n` copy ports.
pub(super) fn service_port_and_copies(
    n: usize,
) -> (TranscriptionCfg, TcpListener, Vec<TcpListener>) {
    let span = FAKE_SERVICE_PORTS.len();
    for step in 0..span {
        let index = (std::process::id() as usize * 104_729 + step) % span;
        let port = FAKE_SERVICE_PORTS.start + u16::try_from(index).unwrap();
        let Ok(service) = TcpListener::bind(("127.0.0.1", port)) else {
            continue;
        };
        let base = service.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{base}/v1");
        let ports = ServiceAddress::parse(&base_url).unwrap().copy_ports();
        let copies: Result<Vec<_>, _> = ports[..n]
            .iter()
            .map(|port| TcpListener::bind(("127.0.0.1", *port)))
            .collect();
        if let Ok(copies) = copies {
            return (
                TranscriptionCfg { base_url, ..TranscriptionCfg::default() },
                service,
                copies,
            );
        }
    }
    panic!("no free service port with free copy ports in {FAKE_SERVICE_PORTS:?}");
}

/// Whether `text` names this process as the copy pool does: its name (the start of it, as
/// `lsof` may cut it) and its pid.
fn names_this_process(text: &str) -> bool {
    let exe = std::env::current_exe().unwrap();
    let name: String = exe
        .file_name()
        .unwrap()
        .to_string_lossy()
        .chars()
        .take(9)
        .collect();
    let pid = format!(" (pid {})", std::process::id());
    text.contains(&name) && text.contains(&pid)
}

#[test]
fn should_transcribe_on_the_copies_when_another_program_holds_the_service_port() {
    // Given
    let (cfg, service, listeners) = service_port_and_copies(2);
    let foreign = ForeignProgram::on(service);
    let copies: Vec<FakeCopy> = listeners
        .into_iter()
        .map(|listener| spawn_copy(listener, vec![CopyTurn::Answer]))
        .collect();
    // When
    let transcript = transcribe(cfg);
    // Then
    assert_eq!(transcript.unwrap().as_deref(), Some("hello"));
    assert_eq!(
        foreign.transcriptions(),
        0,
        "the other program got a transcription request"
    );
    assert_eq!(copies.iter().map(FakeCopy::requests).sum::<usize>(), 1);
}

#[test]
fn should_send_nothing_to_another_program_on_a_copy_port() {
    // Given
    let (cfg, mut listeners) = free_copies(3);
    let foreign = ForeignProgram::on(listeners.pop().unwrap());
    let copies: Vec<FakeCopy> = listeners
        .into_iter()
        .map(|listener| spawn_copy(listener, vec![CopyTurn::Answer]))
        .collect();
    // When
    let transcripts: Vec<_> = (0..3).map(|_| transcribe(cfg.clone())).collect();
    // Then
    for transcript in transcripts {
        assert_eq!(transcript.unwrap().as_deref(), Some("hello"));
    }
    assert_eq!(
        foreign.transcriptions(),
        0,
        "the other program got a transcription request"
    );
    assert_eq!(copies.iter().map(FakeCopy::requests).sum::<usize>(), 3);
}

#[test]
fn should_log_the_program_on_the_service_port_once() {
    // Given
    let (cfg, service, listeners) = service_port_and_copies(1);
    let port = service.local_addr().unwrap().port();
    let _foreign = ForeignProgram::on(service);
    let _copy = spawn_copy(
        listeners.into_iter().next().unwrap(),
        vec![CopyTurn::Answer],
    );
    capturing_logger();
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    // When
    transcriber.transcribe(&half_second()).unwrap();
    transcriber.transcribe(&half_second()).unwrap();
    // Then
    let held = format!("port {port} of the service at http://127.0.0.1:{port}/v1 is held by ");
    let warnings = logged_by_any_thread(&held);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(names_this_process(&warnings[0]), "{}", warnings[0]);
}

#[test]
fn should_fail_naming_the_program_on_the_service_port_when_no_copy_answers() {
    // Given
    let (cfg, service, mut listeners) = service_port_and_copies(1);
    let port = service.local_addr().unwrap().port();
    let _foreign = ForeignProgram::on(service);
    let copy = voicepuncher_test_steps::loopback::WhisperUntilClosed::on(listeners.remove(0));
    let mut transcriber = OpenAiTranscriber::connect(cfg)
        .unwrap()
        .with_recovery_wait(super::copy_recovery::SHORT_RECOVERY_WAIT);
    copy.close();
    // When
    let error = transcriber.transcribe(&half_second()).unwrap_err();
    // Then
    let taken = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<voicepuncher::core::domain::ServicePortTaken>())
        .unwrap_or_else(|| panic!("the error does not name the port's holder: {error:#}"));
    assert_eq!(taken.0.port, port);
    assert!(names_this_process(&taken.0.program), "{taken:?}");
}
