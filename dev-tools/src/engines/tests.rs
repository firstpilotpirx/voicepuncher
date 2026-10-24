//! The engine lock, the relocation of a file and the bundle's expected keys.

use super::lock::EngineLock;
use super::relocate::{arguments, blank, references};
use super::sources::{verify_in, Source, Sources};
use super::{accepted, bundle, staging::crates};
use std::collections::BTreeMap;
use std::path::PathBuf;

const LOCK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../app/packaging/engines.lock"
));

#[test]
fn should_read_every_section_of_the_engine_lock() {
    let lock = EngineLock::parse(LOCK, "engines.lock").unwrap();
    for section in [
        "whisper",
        "whisper.formulae",
        "whisper.from",
        "whisper.sha256",
        "command-spotting",
        "licenses",
    ] {
        assert!(lock.sections.contains_key(section), "no [{section}]");
    }
    assert_eq!(lock.sections["whisper"]["program"], "whisper-server");
    assert_eq!(
        lock.sections["whisper.from"]["libggml.0.dylib"],
        "ggml/lib/libggml.0.dylib"
    );
    assert_eq!(
        lock.sections["whisper.from"].len(),
        lock.sections["whisper.sha256"].len()
    );
}

#[test]
fn should_refuse_a_line_that_is_no_key_value_pair() {
    let refused = EngineLock::parse("[whisper]\nsource homebrew\n", "engines.lock").unwrap_err();
    assert_eq!(refused, "engines.lock line 2: not a key = \"value\" line");
}

#[test]
fn should_read_a_file_from_its_formula() {
    let formulae = BTreeMap::from([("ggml".to_string(), "0.27.0".to_string())]);
    let cellar = Source { cellar: PathBuf::from("/c"), formulae };
    assert_eq!(
        cellar.path("ggml/lib/libggml.0.dylib"),
        PathBuf::from("/c/ggml/0.27.0/lib/libggml.0.dylib")
    );
}

const OTOOL_L: &str = "/x/libwhisper.1.dylib:\n\t/opt/homebrew/opt/ggml/lib/libggml.0.dylib (compatibility version 0.0.0)\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\n";
const OTOOL_RPATHS: &str = "Load command 20\n          cmd LC_RPATH\n      cmdsize 40\n         path /opt/homebrew/lib (offset 12)\n";

#[test]
fn should_point_every_homebrew_reference_into_the_engine_folder() {
    assert_eq!(
        references(OTOOL_L),
        [
            "/opt/homebrew/opt/ggml/lib/libggml.0.dylib",
            "/usr/lib/libSystem.B.dylib"
        ]
    );
    assert_eq!(
        arguments("libwhisper.1.dylib", "DYLIB", OTOOL_L, OTOOL_RPATHS),
        [
            "-change",
            "/opt/homebrew/opt/ggml/lib/libggml.0.dylib",
            "@rpath/libggml.0.dylib",
            "-id",
            "@rpath/libwhisper.1.dylib",
            "-delete_rpath",
            "/opt/homebrew/lib",
            "-add_rpath",
            "@loader_path"
        ]
    );
    let loader = "cmd LC_RPATH\n cmdsize 32\n path @loader_path (offset 12)\n";
    assert!(arguments("whisper-server", "EXECUTE", "", loader).is_empty());
}

#[test]
fn should_blank_every_compiled_in_backend_folder() {
    let mut bytes = b"a/c/ggml/libexec\0b/c/ggml/libexec\0".to_vec();
    assert_eq!(blank(&mut bytes, &["/c/ggml/libexec".to_string()]), 2);
    assert_eq!(
        bytes,
        b"a\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0b\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec()
    );
}

#[test]
fn should_read_the_version_of_each_locked_crate() {
    let found = crates("[[package]]\nname = \"sherpa-onnx\"\nversion = \"1.12.0\"\nsource = \"x\"\n\n[[package]]\nname = \"a\"\nversion = \"2.0.0\"\n");
    assert_eq!(found.get("sherpa-onnx").map(String::as_str), Some("1.12.0"));
    assert_eq!(found.get("a").map(String::as_str), Some("2.0.0"));
}

#[test]
fn should_expect_the_app_version_in_the_bundle() {
    let keys = bundle::expected_keys("0.9.1");
    assert!(keys.contains(&("CFBundleVersion", "0.9.1".to_string())));
    assert!(keys.contains(&("LSUIElement", "true".to_string())));
    assert_eq!(keys.len(), 11);
}

const WRONG_SUM: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn should_refuse_a_homebrew_file_whose_checksum_differs_from_the_lock() {
    let cellar = tempfile::tempdir().unwrap();
    let program = cellar.path().join("whisper-cpp/1.9.5/bin/whisper-server");
    std::fs::create_dir_all(program.parent().unwrap()).unwrap();
    std::fs::write(&program, "the installed program").unwrap();
    let found = super::sha256(&program).unwrap();
    let lock = EngineLock::parse(
        &format!(
            "[whisper]\nsource = \"homebrew\"\n[whisper.formulae]\nwhisper-cpp = \"1.9.5\"\n\
             [whisper.from]\nwhisper-server = \"whisper-cpp/bin/whisper-server\"\n\
             [whisper.sha256]\nwhisper-server = \"{WRONG_SUM}\"\n"
        ),
        "engines.lock",
    )
    .unwrap();
    let seed = Sources { cellar: Some(cellar.path().to_path_buf()), ..Sources::default() };
    let sources = verify_in(seed, &lock, "engines.lock").unwrap();
    assert_eq!(
        sources.problems,
        [format!(
            "whisper: whisper-server ({}) has sha256 {found}, engines.lock expects {WRONG_SUM}",
            program.display()
        )]
    );
    let refused = accepted(sources, "engines.lock").err().unwrap();
    assert!(refused.starts_with("no engine staged"), "{refused}");
}
