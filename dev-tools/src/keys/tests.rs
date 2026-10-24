//! The keys as the former Perl runner computed them for a fixture repository (golden values), and how edits
//! move them.

use super::{compute, Inputs};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const TOOLCHAIN: &str = "rustc 1.88.0 (fake)\nhost: aarch64-apple-darwin\ncargo 1.88.0 (fake)\ncargo-nextest 0.9.146\njust 1.59.0\n25A1\n";
const TABLE: &str = "openspec/*                                     none
docs/*                                         code
*.md                                           none
app/src/adapters/secondary/textrefinement/*    e2e-smoke-refinement,refine-eval,accuracy
command-spotting/*                             e2e-smoke-voice,accuracy
";
const MODEL: &str = "/tmp/voicepuncher-dev-tools-golden.bin";

const FILES: [(&str, &str); 18] = [
    ("Cargo.toml", "[workspace]\nmembers = [\"app\", \"command-spotting\", \"copy-pool\", \"e2e-test\"]\n\n[workspace.package]\nversion = \"1.0.0\""),
    ("Cargo.lock", "[[package]]\nname = \"voicepuncher\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"voicepuncher-copy-pool\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\""),
    ("app/Cargo.toml", "[package]\nname = \"voicepuncher\"\nversion = \"1.0.0\"\n\n[dependencies]\nvoicepuncher-copy-pool = { path = \"../copy-pool\" }\n\n[dev-dependencies]\nvoicepuncher-e2e-test = { path = \"../e2e-test\" }"),
    ("app/src/main.rs", "fn main() {}"),
    ("app/src/a file.rs", "pub fn spaced() {}"),
    ("app/src/adapters/secondary/textrefinement/openai/client.rs", "fn refine() {}"),
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

const GOLDEN: [(&str, &str); 11] = [
    (
        "fmt",
        "9db772183a8622d255937ba6bcc6b0e761a90531d184be6fe356dfeab164567f",
    ),
    (
        "test",
        "99fbcb63f7df0219488931c4f449b58679eeac6340b01d9662b9e1908014a945",
    ),
    (
        "e2e-smoke-voice",
        "28f57c50c6af0b9f90d448e4a6f228b35ff2cd981ebb9ab5663865351848d9d0",
    ),
    (
        "e2e-smoke-refinement",
        "69295a77a8848542c4a7706000a7cfb1d900ed7e1f9dbe1d5b43ab3fbcc4b6fe",
    ),
    (
        "refine-eval",
        "6a788299c7b6fe52fc489a6aaed447ba17445d5ce382af1420311df565ce61d4",
    ),
    (
        "e2e-quick",
        "ad73d555e264073f6a79500fe67bf8f8543d5b8336e11b1a4f1fb6c3ecd8268b",
    ),
    (
        "e2e-bench",
        "ad3641ac868f5432a114198d8f5ca571f041bf68fc750de455d4da166321ad8b",
    ),
    (
        "test@app",
        "2ef8c195d96fdedb8865828da8d488cb5aa399815a3c468b9e8622e6d8c76a4f",
    ),
    (
        "test@command-spotting",
        "e938e79aa79bb1a5ac4e16a61fc9090996ee192a1d7c30b568d442faf74bb428",
    ),
    (
        "test@copy-pool",
        "ec5b55e4c5d434315c6b193c03c52ff5312fb971017a4f95fba29896d0d6da5c",
    ),
    (
        "test@e2e-test",
        "4325a88e54e70a8b4654cf6d9bd06e8d5017b6e8a872853eaaaaf3f897bec645",
    ),
];

fn put(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, format!("{text}\n")).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

/// The fixture repository the golden keys were computed on.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in FILES {
        put(dir.path(), path, text);
    }
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["add", "Cargo.toml", "app"]);
    git(
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
        ("VOICEPUNCHER_LLM_MODEL", MODEL),
        ("NEXTEST_PROFILE", "ci"),
        ("OTHER", "1"),
        ("HOME", "/Users/x"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect();
    let servers: [(&str, Option<PathBuf>); 2] = [("whisper-server", None), ("llama-server", None)];
    Inputs::new(TOOLCHAIN.into(), &env, &servers, TABLE.into())
}

fn keys(root: &Path, stages: &[&str]) -> BTreeMap<String, String> {
    let stages: Vec<String> = stages.iter().map(|stage| (*stage).to_string()).collect();
    compute(root, &stages, &inputs())
        .unwrap()
        .into_iter()
        .collect()
}

const STAGES: [&str; 8] = [
    "fmt",
    "test",
    "e2e-smoke-voice",
    "e2e-smoke-refinement",
    "refine-eval",
    "e2e-quick",
    "e2e-bench",
    "test@members",
];

#[test]
fn should_compute_the_keys_the_former_runner_computed() {
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
