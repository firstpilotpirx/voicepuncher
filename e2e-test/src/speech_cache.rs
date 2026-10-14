//! Speech synthesized by macOS `say`, kept on the machine: each phrase is synthesized once
//! and read back on every later run, by every checkout. `say` costs about a second per
//! call, even for two words, and slows down when called in parallel.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{bail, Context, Result};

use crate::speech::read_wav;

/// Overrides the cache directory.
const CACHE_DIR_ENV: &str = "VOICEPUNCHER_SPEECH_CACHE";
/// The cache directory under the home directory.
const CACHE_DIR_IN_HOME: &str = ".cache/voicepuncher/speech";
/// What `say` writes, the format every test reads.
const AUDIO_FORMAT: [&str; 2] = ["--file-format=WAVE", "--data-format=LEI16@16000"];
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// What `say` is asked to speak; the voice and the rate default to the system's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SayRequest<'a> {
    pub voice: Option<&'a str>,
    pub words_per_minute: Option<u32>,
    pub text: &'a str,
}

/// 16 kHz mono 16-bit speech for `request`, synthesized by `say` once per machine.
pub fn synthesized(request: &SayRequest) -> Result<Vec<i16>> {
    SpeechCache::on_this_machine()?.speech(request, say_into)
}

/// Synthesized speech by request, in one directory.
pub struct SpeechCache {
    dir: PathBuf,
    /// The macOS build: an update may change the voices.
    system: String,
}

impl SpeechCache {
    pub fn new(dir: PathBuf, system: String) -> Self {
        Self { dir, system }
    }

    /// The machine's cache: `VOICEPUNCHER_SPEECH_CACHE`, or `~/.cache/voicepuncher/speech`.
    pub fn on_this_machine() -> Result<Self> {
        let dir = match std::env::var_os(CACHE_DIR_ENV) {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?)
                .join(CACHE_DIR_IN_HOME),
        };
        Ok(Self::new(dir, macos_build()?.to_owned()))
    }

    /// `request`'s speech from the cache; on a miss `synthesize` writes it into the file
    /// it is given, which then joins the cache whole, so a parallel reader never sees half.
    pub fn speech(
        &self,
        request: &SayRequest,
        synthesize: impl FnOnce(&SayRequest, &Path) -> Result<()>,
    ) -> Result<Vec<i16>> {
        let path = self.dir.join(format!("{:016x}.wav", self.key(request)));
        if !path.is_file() {
            std::fs::create_dir_all(&self.dir)
                .with_context(|| format!("create {}", self.dir.display()))?;
            let partial = tempfile::Builder::new()
                .suffix(".wav")
                .tempfile_in(&self.dir)?;
            synthesize(request, partial.path())?;
            partial.persist(&path)?;
        }
        read_wav(&path)
    }

    /// FNV-1a of everything that shapes the speech: stable across runs and Rust versions.
    fn key(&self, request: &SayRequest) -> u64 {
        let rate = request.words_per_minute.map(|r| r.to_string());
        let fields = [
            self.system.as_str(),
            request.voice.unwrap_or(""),
            rate.as_deref().unwrap_or(""),
            AUDIO_FORMAT[1],
            request.text,
        ];
        fields
            .iter()
            .flat_map(|field| field.bytes().chain([0]))
            .fold(FNV_OFFSET_BASIS, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
            })
    }
}

/// Runs `say` for `request`, writing 16 kHz mono 16-bit WAVE to `path`.
pub fn say_into(request: &SayRequest, path: &Path) -> Result<()> {
    let mut say = Command::new("say");
    if let Some(voice) = request.voice {
        say.args(["-v", voice]);
    }
    if let Some(rate) = request.words_per_minute {
        say.arg("-r").arg(rate.to_string());
    }
    let status = say
        .args(AUDIO_FORMAT)
        .arg("-o")
        .arg(path)
        .arg(request.text)
        .status()
        .context("failed to run macOS 'say'")?;
    if !status.success() {
        bail!("'say' failed to synthesize speech ({status})");
    }
    Ok(())
}

fn macos_build() -> Result<&'static str> {
    static BUILD: OnceLock<String> = OnceLock::new();
    if let Some(build) = BUILD.get() {
        return Ok(build);
    }
    let output = Command::new("sw_vers")
        .arg("-buildVersion")
        .output()
        .context("failed to run 'sw_vers'")?;
    let build = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(BUILD.get_or_init(|| build))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::speech::write_wav;

    const SYSTEM: &str = "25A100";
    const SPEECH: [i16; 4] = [0, 100, -100, 0];

    fn request(
        voice: Option<&'static str>,
        rate: Option<u32>,
        text: &'static str,
    ) -> SayRequest<'static> {
        SayRequest {
            voice,
            words_per_minute: rate,
            text,
        }
    }

    /// Speech from `cache`, counting how often it had to be synthesized.
    fn speech_counted(cache: &SpeechCache, request: &SayRequest, made: &Cell<usize>) -> Vec<i16> {
        cache
            .speech(request, |_, path| {
                made.set(made.get() + 1);
                write_wav(path, &SPEECH)
            })
            .unwrap()
    }

    #[test]
    fn should_synthesize_a_phrase_once() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let cache = SpeechCache::new(dir.path().to_owned(), SYSTEM.to_owned());
        let made = Cell::new(0);
        let said = request(Some("Eddy"), None, "press enter");
        // When
        let first = speech_counted(&cache, &said, &made);
        let second = speech_counted(&cache, &said, &made);
        // Then
        assert_eq!(made.get(), 1);
        assert_eq!(first, SPEECH);
        assert_eq!(second, SPEECH);
    }

    #[test]
    fn should_keep_voice_rate_and_text_apart() {
        // Given
        let dir = tempfile::tempdir().unwrap();
        let cache = SpeechCache::new(dir.path().to_owned(), SYSTEM.to_owned());
        let made = Cell::new(0);
        let requests = [
            request(None, None, "press enter"),
            request(Some("Eddy"), None, "press enter"),
            request(Some("Eddy"), Some(120), "press enter"),
            request(Some("Eddy"), Some(120), "clear all text"),
        ];
        // When
        for said in &requests {
            speech_counted(&cache, said, &made);
        }
        // Then
        assert_eq!(made.get(), requests.len());
    }
}
