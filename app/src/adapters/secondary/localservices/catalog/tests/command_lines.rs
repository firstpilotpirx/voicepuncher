//! The app and the service scripts (`justfile`) start each service with the same arguments.

use std::fs;

use super::super::*;

fn justfile() -> String {
    fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../justfile")).unwrap()
}

/// The words of `text` from `start` up to `end`, without line continuations.
fn words_between(text: &str, start: &str, end: &str) -> Vec<String> {
    let from = text
        .find(start)
        .unwrap_or_else(|| panic!("no {start} in the justfile"));
    let rest = &text[from + start.len()..];
    let to = rest
        .find(end)
        .unwrap_or_else(|| panic!("no {end} after {start}"));
    rest[..to]
        .split_whitespace()
        .filter(|word| *word != "\\")
        .map(|word| word.trim_matches('"').to_string())
        .collect()
}

/// Where the service scripts' arguments of `service` start and end, and their port.
fn script_bounds(service: &str) -> (&'static str, &'static str, &'static str) {
    match service {
        WHISPER => ("whisper_cmd=(whisper-server", ")\n", "$2"),
        LLAMA => (
            "\"http://127.0.0.1:{{llm_port}}/health\" llama-server",
            "\n",
            "{{llm_port}}",
        ),
        _ => (
            "\"{{kws_binary}}\"",
            "${VOICEPUNCHER_KWS_SAVE_SPEECH_DIR",
            "{{kws_port}}",
        ),
    }
}

/// The catalog's arguments of `service`, with the scripts' variables, port 0 and 0 threads.
fn catalog_args(service: &str) -> Vec<String> {
    match service {
        WHISPER => whisper_args("$1", 0, 0, "$VOICEPUNCHER_VAD_MODEL"),
        LLAMA => llama_args("$VOICEPUNCHER_LLM_MODEL", 0),
        _ => command_spotting_args(
            0,
            "$VOICEPUNCHER_KWS_MODEL_DIR",
            "$VOICEPUNCHER_KWS_VAD_MODEL",
            "{{command_stt_url}}",
        ),
    }
}

/// The service scripts' arguments of `service` in `text`, port and threads as 0.
fn script_args(service: &str, text: &str) -> Vec<String> {
    let (start, end, port) = script_bounds(service);
    words_between(text, start, end)
        .into_iter()
        .map(|word| {
            if word == port || word == "$3" {
                "0".into()
            } else {
                word
            }
        })
        .collect()
}

/// The services whose arguments differ between the service scripts in `text` and the catalog.
fn drifted(text: &str) -> Vec<&'static str> {
    [WHISPER, LLAMA, COMMAND_SPOTTING]
        .into_iter()
        .filter(|service| script_args(service, text) != catalog_args(service))
        .collect()
}

#[test]
fn should_start_every_service_with_the_arguments_of_the_service_scripts() {
    // Given
    let text = justfile();
    // When
    let drifted = drifted(&text);
    // Then
    assert!(
        drifted.is_empty(),
        "arguments differ from the justfile for {drifted:?}"
    );
}

#[test]
fn should_name_the_service_whose_arguments_changed_in_one_place_only() {
    // Given
    let text = justfile().replace("-c 4096 -np 1", "-c 4096 -np 2");
    // When
    let drifted = drifted(&text);
    // Then
    assert_eq!(drifted, vec![LLAMA]);
}
