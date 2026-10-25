//! The keys for a fixture repository (golden values, first those the former Perl runner computed), and how
//! edits move them.

use super::{compute, Inputs};
use crate::process::git_ok;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const TOOLCHAIN: &str = "rustc 1.88.0 (fake)\nhost: aarch64-apple-darwin\ncargo 1.88.0 (fake)\ncargo-nextest 0.9.146\njust 1.59.0\n25A1\n";
const TABLE: &str = "openspec/*                                     none
docs/*                                         code
*.md                                           none
app/src/adapters/secondary/transcription/*     e2e-smoke-speech,accuracy
command-spotting/*                             e2e-smoke-voice,accuracy
";
const MODEL: &str = "/tmp/voicepuncher-dev-tools-golden.bin";

const FILES: [(&str, &str); 18] = [
    ("Cargo.toml", "[workspace]\nmembers = [\"app\", \"command-spotting\", \"copy-pool\", \"e2e-test\"]\n\n[workspace.package]\nversion = \"1.0.0\""),
    ("Cargo.lock", "[[package]]\nname = \"voicepuncher\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"voicepuncher-copy-pool\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\""),
    ("app/Cargo.toml", "[package]\nname = \"voicepuncher\"\nversion = \"1.0.0\"\n\n[dependencies]\nvoicepuncher-copy-pool = { path = \"../copy-pool\" }\n\n[dev-dependencies]\nvoicepuncher-e2e-test = { path = \"../e2e-test\" }"),
    ("app/src/main.rs", "fn main() {}"),
    ("app/src/a file.rs", "pub fn spaced() {}"),
    ("app/src/adapters/secondary/transcription/openai/client.rs", "fn transcribe() {}"),
    ("command-spotting/Cargo.toml", "[package]\nname = \"voicepuncher-command-spotting\"\n\n[dependencies]\nvoicepuncher-copy-pool = { path = \"../copy-pool\" }"),
    ("command-spotting/src/main.rs", "fn main() {}"),
    ("copy-pool/Cargo.toml", "[package]\nname = \"voicepuncher-copy-pool\""),
    ("copy-pool/src/lib.rs", "pub fn pool() {}"),
    ("e2e-test/Cargo.toml", "[package]\nname = \"voicepuncher-e2e-test\"\n\n[dependencies]\nvoicepuncher = { path = \"../app\" }\n\n[dev-dependencies]\nvoicepuncher-command-spotting = { path = \"../command-spotting\" }"),
    ("e2e-test/src/lib.rs", "pub fn e2e() {}"),
    ("docs/notes.md", "notes"),
    ("README.md", "readme"),
    ("openspec/change.md", "change"),
    ("justfile", "test:"),
    (".gitignore", "ignored/"),
    ("ignored/x.txt", "ignored"),
];

const GOLDEN: [(&str, &str); 10] = [
    (
        "fmt",
        "03b5edc5f83536c032d2e82255a967f1ae9eaa66e2b7eab709b0c62038becacb",
    ),
    (
        "test",
        "d6938136858384a0a3cf1d17e1092f15c9dae698660c0f6ff40e61279b9330d3",
    ),
    (
        "e2e-smoke-voice",
        "f2e617fc8da912a511fc3c360fa4375692d0cea6919b5b152044d2ca2850fcb9",
    ),
    (
        "e2e-smoke-speech",
        "9e88053a5a129f0322c50c84d5cee3d2bf1efbbc5b5913a704b26fbc444981a1",
    ),
    (
        "e2e-quick",
        "12cb424a0cfc5118136e3b5166ed0536ff146f62456d3cdbeae6273938f32a81",
    ),
    (
        "e2e-bench",
        "45432507279b82d7c0976deac6b79b7375f29d15cbfad3372fd44938e83045fc",
    ),
    (
        "test@app",
        "e11252b7436128f12605f67ddb739ed611deb06dbc1bcffe680e62e953f53d20",
    ),
    (
        "test@command-spotting",
        "7708417eeb8f01835bcbe370e747e166db75961ec773e3ae93863a05729b8f87",
    ),
    (
        "test@copy-pool",
        "a2413127ab699f20715ac59bd4e440cb95150cf35ecca332d3f9b5189b6fd31f",
    ),
    (
        "test@e2e-test",
        "92972d27eb1c5376fee63bde452e057f65add80235ade4741151407b284ce96c",
    ),
];

fn put(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, format!("{text}\n")).unwrap();
}

/// The fixture repository the golden keys were computed on.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in FILES {
        put(dir.path(), path, text);
    }
    git_ok(dir.path(), &["init", "-q"]);
    git_ok(dir.path(), &["add", "Cargo.toml", "app"]);
    git_ok(
        dir.path(),
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "init",
        ],
    );
    dir
}

fn model_file() {
    static WRITTEN: std::sync::Once = std::sync::Once::new();
    WRITTEN.call_once(write_model);
}

fn write_model() {
    let file = std::fs::File::create(MODEL).unwrap();
    std::io::Write::write_all(&mut &file, b"weights\n").unwrap();
    let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    file.set_modified(at).unwrap();
}

fn inputs() -> Inputs {
    let env: BTreeMap<String, String> = [
        ("VOICEPUNCHER_STAGE_ROOT", "/repo"),
        ("VOICEPUNCHER_HANG_PROBE", "1"),
        ("VOICEPUNCHER_TEST_STT_PORT", "1"),
        ("VOICEPUNCHER_FAIL_FAST", "1"),
        ("RUSTFLAGS", "-Dwarnings"),
        ("CUCUMBER_TAGS", "x"),
        ("CARGO_TERM_COLOR", "always"),
        ("VOICEPUNCHER_WHISPER_MODEL", MODEL),
        ("NEXTEST_PROFILE", "ci"),
        ("OTHER", "1"),
        ("HOME", "/Users/x"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect();
    let servers: [(&str, Option<PathBuf>); 1] = [("whisper-server", None)];
    Inputs::new(TOOLCHAIN.into(), &env, &servers, TABLE.into())
}

fn keys(root: &Path, stages: &[&str]) -> BTreeMap<String, String> {
    let stages: Vec<String> = stages.iter().map(|stage| (*stage).to_string()).collect();
    compute(root, &stages, &inputs())
        .unwrap()
        .into_iter()
        .collect()
}

const STAGES: [&str; 7] = [
    "fmt",
    "test",
    "e2e-smoke-voice",
    "e2e-smoke-speech",
    "e2e-quick",
    "e2e-bench",
    "test@members",
];

#[test]
fn should_compute_the_golden_keys_of_the_fixture() {
    model_file();
    let repo = fixture();
    let found = keys(repo.path(), &STAGES);
    let golden: BTreeMap<String, String> = GOLDEN
        .iter()
        .map(|(stage, key)| ((*stage).to_string(), (*key).to_string()))
        .collect();
    assert_eq!(found, golden);
}

#[test]
fn should_move_only_the_keys_of_the_stages_an_edit_feeds() {
    model_file();
    let repo = fixture();
    let before = keys(repo.path(), &STAGES);
    put(
        repo.path(),
        "command-spotting/src/main.rs",
        "fn main() { spot() }",
    );
    let after = keys(repo.path(), &STAGES);
    let moved: Vec<&str> = before
        .keys()
        .filter(|stage| before[*stage] != after[*stage])
        .map(String::as_str)
        .collect();
    assert_eq!(
        moved,
        [
            "e2e-bench",
            "e2e-quick",
            "e2e-smoke-voice",
            "fmt",
            "test",
            "test@command-spotting",
            "test@e2e-test"
        ]
    );
}

#[test]
fn should_ignore_a_version_bump_for_the_model_stages() {
    model_file();
    let repo = fixture();
    let before = keys(repo.path(), &["e2e-quick", "fmt"]);
    put(
        repo.path(),
        "app/Cargo.toml",
        &FILES[2].1.replace("1.0.0", "1.1.0"),
    );
    let after = keys(repo.path(), &["e2e-quick", "fmt"]);
    assert_eq!(before["e2e-quick"], after["e2e-quick"]);
    assert_ne!(before["fmt"], after["fmt"]);
}

#[test]
fn should_refuse_a_part_that_is_no_member() {
    let repo = fixture();
    let refused = compute(repo.path(), &["test@nothing".to_string()], &inputs()).unwrap_err();
    assert_eq!(refused, "test@nothing: no workspace member nothing");
}
