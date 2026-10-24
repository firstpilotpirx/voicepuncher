//! The selections and the needs (former `_test-gates`): closed needs filtered out and counted, open ones left in.

use super::gates::{closed, skipped_lines, with_lock, without, Closed};
use super::Selection;
use std::collections::BTreeMap;

const TABLES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../.config/stages.toml"
));

fn needs() -> String {
    crate::context::table(TABLES, "test_needs").unwrap()
}

/// Every need met, but the variables named (set empty) and the ones given.
fn env(changed: &[(&str, &str)], corpus: &std::path::Path) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = [
        "VOICEPUNCHER_TRANSCRIPTION_URL",
        "VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL",
        "VOICEPUNCHER_COMMAND_SPOTTING_URL",
        "VOICEPUNCHER_KWS_MODEL_DIR",
        "VOICEPUNCHER_KWS_VAD_MODEL",
        "VOICEPUNCHER_VOICE_SAMPLES",
        "VOICEPUNCHER_DESKTOP_IT",
        "VOICEPUNCHER_REAL_KEYBOARD_IT",
        "VOICEPUNCHER_HANG_PROBE",
        "MACHINE_LOCK",
    ]
    .iter()
    .map(|name| ((*name).to_string(), "1".to_string()))
    .collect();
    env.insert(
        "VOICEPUNCHER_BENCH_CORPUS".into(),
        corpus.to_string_lossy().into_owned(),
    );
    for (name, value) in changed {
        env.insert((*name).into(), (*value).into());
    }
    env
}

fn closed_in(env: &BTreeMap<String, String>) -> Vec<Closed> {
    closed(&needs(), &|name| {
        env.get(name).filter(|value| !value.is_empty()).cloned()
    })
}

fn corpus() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("manifest.jsonl"), "").unwrap();
    dir
}

const LISTED: &str = "voicepuncher::it transcription_openai::needs_stt::should_transcribe
voicepuncher::it voice_samples::needs_kws::needs_voice_samples::should_spot
voicepuncher::it voice_samples::needs_kws::needs_voice_samples::should_spot_more
";

#[test]
fn should_leave_the_filter_as_it_was_with_every_need_met() {
    let corpus = corpus();
    assert_eq!(
        without("binary(=it)", &closed_in(&env(&[], corpus.path()))),
        "binary(=it)"
    );
}

#[test]
fn should_filter_a_closed_need_out_and_count_it_with_its_reason() {
    let corpus = corpus();
    let closed = closed_in(&env(
        &[
            ("VOICEPUNCHER_TRANSCRIPTION_URL", ""),
            ("VOICEPUNCHER_VOICE_SAMPLES", ""),
        ],
        corpus.path(),
    ));
    assert_eq!(
        without("binary(=it)", &closed),
        "(binary(=it)) & not (test(/needs_stt::/) | test(/needs_voice_samples::/))"
    );
    let lines = skipped_lines(LISTED, &closed);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "● skipped 1 test(s): needs_stt — no speech recognition service (the model stages start one)");
    assert!(lines[1].starts_with(
        "● skipped 2 test(s): needs_voice_samples — no recordings of the user's voice"
    ));
}

#[test]
fn should_close_a_need_when_one_of_its_variables_is_empty_or_its_file_is_missing() {
    let corpus = corpus();
    let missing = corpus.path().join("none");
    let env = env(
        &[
            ("VOICEPUNCHER_KWS_VAD_MODEL", ""),
            ("VOICEPUNCHER_BENCH_CORPUS", &missing.to_string_lossy()),
        ],
        corpus.path(),
    );
    let filter = without("x", &closed_in(&env));
    assert!(filter.contains("test(/needs_kws_models::/)"), "{filter}");
    assert!(filter.contains("test(/needs_corpus::/)"), "{filter}");
    assert!(skipped_lines(LISTED, &closed_in(&env)).is_empty());
}

#[test]
fn should_build_each_selection_narrowed_to_a_rerun_and_without_the_frozen_tests() {
    let modules = ["placed_commands".to_string(), "voice_forms".to_string()];
    assert_eq!(
        Selection::Smoke(&modules).narrowed(None, None),
        "(binary(=it) & test(/^placed_commands::/)) | (binary(=it) & test(/^voice_forms::/)) | binary(=cucumber_e2e)"
    );
    assert_eq!(
        Selection::It(&modules[..1]).narrowed(Some("test(x)"), Some("test(y)")),
        "((binary(=it) & (test(/^placed_commands::/))) & (test(x))) & not (test(y))"
    );
    assert_eq!(
        Selection::Test.narrowed(Some("test(x)"), Some("test(y)")),
        "not binary_id(voicepuncher-e2e-test::cucumber_e2e) & (test(x)) & not (test(y))"
    );
}

#[test]
fn should_skip_the_machine_lock_tests_without_the_lock() {
    let corpus = corpus();
    let mut env = env(&[("VOICEPUNCHER_NO_MACHINE_LOCK", "1")], corpus.path());
    env.remove("MACHINE_LOCK");
    let closed = closed_in_with_lock(&env);
    assert_eq!(
        without("binary(=dev-tools)", &closed),
        "(binary(=dev-tools)) & not (test(/needs_machine_lock::/))"
    );
    let listed = "voicepuncher-dev-tools::bin/dev-tools locks::tests::needs_machine_lock::should_find_the_wrapper_it_runs\n";
    assert_eq!(
        skipped_lines(listed, &closed),
        ["● skipped 1 test(s): needs_machine_lock — no machine lock (VOICEPUNCHER_NO_MACHINE_LOCK=1, the hosted CI)"]
    );
}

#[test]
fn should_run_the_machine_lock_tests_with_the_default_lock_and_no_variable() {
    let corpus = corpus();
    let mut env = env(&[], corpus.path());
    env.remove("MACHINE_LOCK");
    assert!(
        !closed_in_with_lock(&env)
            .iter()
            .any(|closed| closed.need == "needs_machine_lock"),
        "the default lock closed needs_machine_lock"
    );
}

fn closed_in_with_lock(env: &BTreeMap<String, String>) -> Vec<Closed> {
    let lookup = |name: &str| env.get(name).filter(|value| !value.is_empty()).cloned();
    closed(&needs(), &|name| with_lock(name, &lookup))
}
