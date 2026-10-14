//! Speech synthesized by macOS `say`, kept on the machine: each phrase is synthesized once
//! and read back on every later run. Same directory and key as the E2E harness's cache
//! (`e2e-test/src/speech_cache.rs`), so both share their entries; the service's tests do not
//! depend on the app crate that the harness pulls in.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const CACHE_DIR_ENV: &str = "VOICEPUNCHER_SPEECH_CACHE";
const CACHE_DIR_IN_HOME: &str = ".cache/voicepuncher/speech";
const AUDIO_FORMAT: [&str; 2] = ["--file-format=WAVE", "--data-format=LEI16@16000"];
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// `words` spoken by `say` in `voice` (the system's voice when `None`), 16 kHz mono, without
/// any pause.
pub fn spoken_by(voice: Option<&str>, words: &str) -> Vec<f32> {
    let path = cache_dir().join(format!("{:016x}.wav", key(voice, words)));
    if !path.is_file() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let partial = tempfile::Builder::new()
            .suffix(".wav")
            .tempfile_in(path.parent().unwrap())
            .unwrap();
        say_into(voice, words, partial.path());
        partial.persist(&path).unwrap();
    }
    hound::WavReader::open(&path)
        .unwrap()
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / f32::from(i16::MAX))
        .collect()
}

fn cache_dir() -> PathBuf {
    std::env::var_os(CACHE_DIR_ENV).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").expect("HOME is set")).join(CACHE_DIR_IN_HOME),
        PathBuf::from,
    )
}

fn say_into(voice: Option<&str>, words: &str, path: &Path) {
    let mut say = Command::new("say");
    if let Some(voice) = voice {
        say.args(["-v", voice]);
    }
    let status = say
        .args(AUDIO_FORMAT)
        .arg("-o")
        .arg(path)
        .arg(words)
        .status()
        .expect("say runs");
    assert!(
        status.success(),
        "'say' failed to synthesize speech ({status})"
    );
}

/// The E2E harness's key: FNV-1a of the macOS build, voice, rate (none here), format, text.
fn key(voice: Option<&str>, words: &str) -> u64 {
    [
        macos_build(),
        voice.unwrap_or(""),
        "",
        AUDIO_FORMAT[1],
        words,
    ]
    .iter()
    .flat_map(|field| field.bytes().chain([0]))
    .fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

/// The macOS build: an update may change the voices.
fn macos_build() -> &'static str {
    static BUILD: OnceLock<String> = OnceLock::new();
    BUILD.get_or_init(|| {
        let output = Command::new("sw_vers")
            .arg("-buildVersion")
            .output()
            .expect("sw_vers runs");
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    })
}
