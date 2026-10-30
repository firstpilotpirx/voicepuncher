use super::command_lines::{command_spotting_args, streaming_args, whisper_args};
use super::e2e::{unlisted, watched_ports, Watched};
use super::launch::{last_lines, supervisor_args, Service};
use super::ready::{note, port_open, Ready};
use super::start::{checked_copies, dropped_by_build, is_yes};
use super::supervise::{exit_line, health_line, next_delay, Fails};
use super::*;
use std::time::Duration;

fn speech(port: u32, copies: &str) -> Speech {
    Speech {
        name: "whisper-server",
        port,
        copies: copies.into(),
        model_var: "VOICEPUNCHER_WHISPER_MODEL",
        label: "speech recognition service for dictation",
    }
}

fn settings() -> Settings {
    Settings {
        root: PathBuf::from("/repo"),
        run_dir: PathBuf::from("/home/.voicepuncher/run"),
        log_dir: PathBuf::from("/home/.voicepuncher/logs"),
        dictation: speech(28081, "3"),
        commands: Speech {
            name: "whisper-server-commands",
            copies: "1".into(),
            model_var: "VOICEPUNCHER_COMMAND_WHISPER_MODEL",
            label: "speech recognition service for commands",
            port: 28085,
        },
        kws_port: 28082,
    }
}

#[test]
fn should_put_the_copies_10000_above_the_port() {
    assert_eq!(copy_ports(8081, 3), vec![18082, 18083, 18084]);
    assert_eq!(copy_ports(8085, 4), vec![18086, 18087, 18088, 18089]);
}

#[test]
fn should_put_the_copies_10000_below_a_port_whose_copies_would_pass_65535() {
    assert_eq!(copy_ports(60000, 2), vec![50001, 50002]);
}

#[test]
fn should_accept_1_to_4_copies_only() {
    assert_eq!(checked_copies(&speech(8081, "4")), Ok(4));
    let refused = checked_copies(&speech(8081, "5"));
    assert_eq!(
        refused,
        Err(Stop::Message(
            "speech recognition service for dictation: 5 copies — set 1 to 4: its clients look for at most 4".into()
        ))
    );
    assert!(checked_copies(&speech(8081, "+2")).is_err());
    assert!(checked_copies(&speech(8081, "0")).is_err());
}

#[test]
fn should_start_whisper_with_the_arguments_of_both_speech_recognition_services() {
    let args = whisper_args("m.bin", "18082", "4", "vad.bin");
    assert_eq!(
        args[..6],
        ["-m", "m.bin", "--host", "127.0.0.1", "--port", "18082"]
    );
    assert_eq!(args.last().map(String::as_str), Some("-nlp"));
    assert!(args.windows(2).any(|pair| pair == ["-vm", "vad.bin"]));
}

#[test]
fn should_start_command_spotting_with_its_arguments() {
    assert_eq!(
        command_spotting_args("8082", "kws", "vad.onnx", "http://127.0.0.1:8085/v1").join(" "),
        "--port 8082 --model-dir kws --vad-model vad.onnx --transcription-url http://127.0.0.1:8085/v1"
    );
}

#[test]
fn should_start_command_spotting_with_the_streaming_recognizers_arguments() {
    assert_eq!(
        streaming_args("8082", "en", "ru", "vad.onnx").join(" "),
        "--port 8082 --recognizer streaming --en-model-dir en --ru-model-dir ru --vad-model vad.onnx"
    );
}

#[test]
fn should_name_the_pid_log_and_copy_files_by_binary_and_port() {
    let settings = settings();
    assert_eq!(
        settings.pid_file("whisper-server", 18082),
        PathBuf::from("/home/.voicepuncher/run/whisper-server-18082.pid")
    );
    assert_eq!(
        settings.log_file("voicepuncher-command-spotting", 8082),
        PathBuf::from("/home/.voicepuncher/logs/voicepuncher-command-spotting-8082.log")
    );
    assert_eq!(
        settings.copies_file(8081),
        PathBuf::from("/home/.voicepuncher/run/whisper-server-8081.copies")
    );
    assert_eq!(
        settings.kws_binary(),
        PathBuf::from("/repo/target/release/voicepuncher-command-spotting")
    );
}

#[test]
fn should_give_the_urls_of_the_services() {
    let settings = settings();
    assert_eq!(settings.command_stt_url(), "http://127.0.0.1:28085/v1");
    assert_eq!(settings.kws_url(), "ws://127.0.0.1:28082");
}

#[test]
fn should_select_every_service_in_start_order_when_none_is_named() {
    assert_eq!(selected(&[]), SERVICES.map(String::from).to_vec());
    assert_eq!(
        selected(&["whisper-server-commands".into()]),
        vec!["whisper-server-commands".to_string()]
    );
}

#[test]
fn should_name_the_known_services_for_an_unknown_one() {
    let Stop::Message(message) = unknown("whisper") else {
        unreachable!("unknown gives a message")
    };
    assert_eq!(
        message,
        "unknown service 'whisper' — use whisper-server, whisper-server-commands or \
         voicepuncher-command-spotting"
    );
}

#[test]
fn should_watch_every_test_port_with_the_variable_that_moves_it() {
    let watched = watched_ports(&settings(), "VOICEPUNCHER_TEST");
    let ports: Vec<u32> = watched.iter().map(|w| w.port).collect();
    assert_eq!(ports, vec![28081, 38082, 38083, 38084, 28085, 28082]);
    assert_eq!(
        watched[1],
        Watched {
            port: 38082,
            service: "whisper-server copy".into(),
            variable: "VOICEPUNCHER_TEST_STT_PORT".into()
        }
    );
    assert_eq!(watched[4].variable, "VOICEPUNCHER_TEST_COMMAND_STT_PORT");
}

#[test]
fn should_drop_the_stop_lines_of_services_that_were_not_there() {
    let text = "· no command spotting service started by start-model-servers\n✓ stopped whisper-server (pid 7)\n";
    assert_eq!(unlisted(text), "✓ stopped whisper-server (pid 7)\n");
}

#[test]
fn should_find_a_free_port_closed() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = u32::from(listener.local_addr().unwrap().port());
    assert!(port_open(port));
    drop(listener);
    assert!(!port_open(port));
    assert_eq!(Ready::Port { port }.check(), None);
}

#[test]
fn should_note_how_long_the_first_transcription_took() {
    assert_eq!(
        note(Duration::from_millis(1234)),
        ", first transcription 1234 ms"
    );
}

#[test]
fn should_pass_the_supervisor_its_name_log_health_and_command() {
    let service = Service {
        binary: "whisper-server".into(),
        port: 8081,
        label: "speech recognition service for dictation".into(),
        ready: Ready::Port { port: 8081 },
        health: None,
        command: vec!["whisper-server".into(), "-m".into()],
    };
    let args = supervisor_args(&service, Path::new("/l.log"));
    assert_eq!(
        args,
        [
            "supervise",
            "supervise-whisper-server",
            "/l.log",
            "-",
            "whisper-server",
            "-m"
        ]
    );
}

#[test]
fn should_print_the_last_lines_of_a_log() {
    assert_eq!(last_lines("a\nb\nc\n", 2), "b\nc\n");
    assert_eq!(last_lines("a", 20), "a\n");
}

#[test]
fn should_double_the_restart_wait_up_to_30_s() {
    assert_eq!(next_delay(1), 2);
    assert_eq!(next_delay(16), 30);
    assert_eq!(
        exit_line("whisper-server", 137, 4),
        "… whisper-server exited with code 137 — restarting in 4 s"
    );
}

#[test]
fn should_kill_a_service_after_3_failed_health_checks_once_it_answered() {
    let mut fails = Fails::default();
    let before = [false, false, false, false].map(|answer| fails.record(answer));
    assert_eq!(before, [false; 4]);
    let after = [true, false, false, true, false, false, false].map(|answer| fails.record(answer));
    assert_eq!(after, [false, false, false, false, false, false, true]);
    assert_eq!(
        health_line("voicepuncher-command-spotting", fails.count),
        "… voicepuncher-command-spotting stopped answering its health check (3 checks in a row) — restarting"
    );
}

#[test]
fn should_drop_the_variables_cargo_sets_for_a_test_from_the_build() {
    assert!(dropped_by_build("CARGO_PKG_NAME"));
    assert!(dropped_by_build("CARGO_MANIFEST_DIR"));
    assert!(dropped_by_build("CARGO_TARGET_TMPDIR"));
    assert!(!dropped_by_build("CARGO_HOME"));
    assert!(!dropped_by_build("CARGO_TARGET_DIR"));
}

#[test]
fn should_install_only_on_y() {
    assert!(is_yes("y\n"));
    assert!(is_yes("Y"));
    assert!(!is_yes("yes\n"));
    assert!(!is_yes("\n"));
}

#[test]
fn should_print_a_model_path_as_basename_does() {
    assert_eq!(basename("/m/ggml-small.bin"), "ggml-small.bin");
    assert_eq!(basename("/m/sherpa-onnx-kws/"), "sherpa-onnx-kws");
    assert_eq!(basename(""), "");
}

#[test]
fn should_turn_a_step_into_an_outcome() {
    assert_eq!(outcome(Ok(())), Ok(0));
    assert_eq!(outcome(Err(Stop::Code(3))), Ok(3));
    assert_eq!(outcome(Err(Stop::Message("x".into()))), Err("x".into()));
}
