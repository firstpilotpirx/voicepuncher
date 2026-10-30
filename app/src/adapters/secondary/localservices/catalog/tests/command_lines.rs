//! The app and the service scripts (`dev-tools start-model-servers`) start each service with the same arguments.

use std::fs;

use super::super::*;

/// The command spotting service with the streaming recognizers, compared as a service of its own.
const STREAMING: &str = "voicepuncher-command-spotting --recognizer streaming";

/// The service scripts' command lines: one literal list per service.
fn scripts() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../dev-tools/src/services/command_lines.rs"
    ))
    .unwrap()
}

/// The elements of the list of `fn <function>(` in `text`: a string literal as its text, a parameter as its name.
fn list_of(text: &str, function: &str) -> Vec<String> {
    let start = format!("fn {function}(");
    let from = text
        .find(&start)
        .unwrap_or_else(|| panic!("no {start} in the service scripts"));
    let rest = &text[from..];
    let open = rest
        .find("&[")
        .unwrap_or_else(|| panic!("no list in {function}"));
    let rest = &rest[open + 2..];
    let close = rest
        .find(']')
        .unwrap_or_else(|| panic!("no end of the list in {function}"));
    rest[..close]
        .split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(|word| word.trim_matches('"').to_string())
        .collect()
}

/// The service scripts' arguments of `service` in `text`, the port and threads parameters as 0.
fn script_args(service: &str, text: &str) -> Vec<String> {
    let function = match service {
        WHISPER => "whisper_args",
        STREAMING => "streaming_args",
        _ => "command_spotting_args",
    };
    list_of(text, function)
        .into_iter()
        .map(|word| match word.as_str() {
            "port" | "threads" => "0".into(),
            _ => word,
        })
        .collect()
}

/// The catalog's arguments of `service`, with the scripts' parameter names, port 0 and 0 threads.
fn catalog_args(service: &str) -> Vec<String> {
    match service {
        WHISPER => whisper_args("model", 0, 0, "vad"),
        STREAMING => {
            let models = ["en_model_dir", "ru_model_dir", "vad"].map(String::from);
            streaming_args(0, &models, None)
        }
        _ => command_spotting_args(0, "model_dir", "vad", "commands_url", None),
    }
}

/// The services whose arguments differ between the service scripts in `text` and the catalog.
fn drifted(text: &str) -> Vec<&'static str> {
    [WHISPER, COMMAND_SPOTTING, STREAMING]
        .into_iter()
        .filter(|service| script_args(service, text) != catalog_args(service))
        .collect()
}

#[test]
fn should_start_every_service_with_the_arguments_of_the_service_scripts() {
    // Given
    let text = scripts();
    // When
    let drifted = drifted(&text);
    // Then
    assert!(
        drifted.is_empty(),
        "arguments differ from dev-tools/src/services/command_lines.rs for {drifted:?}"
    );
}

#[test]
fn should_name_the_service_whose_arguments_changed_in_one_place_only() {
    // Given
    let text = scripts().replace("\"2000\"", "\"3000\"");
    // When
    let drifted = drifted(&text);
    // Then
    assert_eq!(drifted, vec![WHISPER]);
}

#[test]
fn should_name_the_streaming_recognizer_when_only_its_arguments_changed() {
    // Given
    let text = scripts().replace("\"--en-model-dir\"", "\"--english-model-dir\"");
    // When
    let drifted = drifted(&text);
    // Then
    assert_eq!(drifted, vec![STREAMING]);
}

#[test]
fn should_pass_the_archive_size_limit_to_the_command_history() {
    // Given
    let mut archive = crate::config::Config::default().archive;
    archive.max_size_mb = 123;
    let home = std::path::Path::new("/Users/someone");
    // When
    let kept = CommandHistoryArgs::of(&archive, home);
    let args = command_spotting_args(0, "model_dir", "vad", "commands_url", kept.as_ref());
    // Then
    let tail = &args[args.len() - 4..];
    let expected = [
        "--command-history-dir",
        "/Users/someone/.voicepuncher/command-history",
        "--command-history-max-mb",
        "123",
    ];
    assert_eq!(tail, expected);
}

#[test]
fn should_pass_no_command_history_when_it_is_turned_off() {
    // Given
    let mut archive = crate::config::Config::default().archive;
    archive.keep_commands = false;
    // When
    let kept = CommandHistoryArgs::of(&archive, std::path::Path::new("/Users/someone"));
    // Then
    assert!(kept.is_none());
}
