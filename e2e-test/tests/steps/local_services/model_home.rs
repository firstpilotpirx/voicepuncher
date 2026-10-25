//! A home of the scenario's own whose `.voicepuncher/models/` holds the models the service
//! scripts download, found where the user keeps them.

use std::ffi::OsStr;
use voicepuncher::test_doubles::prelude::*;

/// The dictation model `just models` downloads.
pub(super) const DICTATION_MODEL: &str = "ggml-large-v3-turbo-q5_0.bin";
/// The commands model `just models` downloads.
pub(super) const COMMAND_MODEL: &str = "ggml-small.bin";
const DICTATION_MODEL_VAR: &str = "VOICEPUNCHER_WHISPER_MODEL";
const COMMAND_MODEL_VAR: &str = "VOICEPUNCHER_COMMAND_WHISPER_MODEL";
/// A file named like a Whisper model, larger than every real one, that is no model: a rule
/// that takes the largest model file would start a service on it, which then fails to load.
const LARGER_FILE: &str = "ggml-larger-than-any-model.bin";
const LARGER_FILE_BYTES: u64 = 2 << 30;
/// The voice activity detection models the services load beside a Whisper model.
const VAD_MODEL_PREFIX: &str = "ggml-silero-";
/// Fills `models` with links to the downloaded models under their download names, beside a
/// larger file that is no model.
pub(super) fn link_models(models: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(models)?;
    for (file, var) in [
        (DICTATION_MODEL, DICTATION_MODEL_VAR),
        (COMMAND_MODEL, COMMAND_MODEL_VAR),
    ] {
        std::os::unix::fs::symlink(downloaded_model(file, var)?, models.join(file))?;
    }
    for vad in vad_models() {
        let file = vad.file_name().context("a VAD model has a file name")?;
        std::os::unix::fs::symlink(&vad, models.join(file))?;
    }
    std::fs::File::create(models.join(LARGER_FILE))?.set_len(LARGER_FILE_BYTES)?;
    Ok(())
}

/// Where a downloaded model is: the run's model variable when it names a file of this name
/// (a checked copy kept elsewhere), else the user's `~/.voicepuncher/` or its `models/`.
fn downloaded_model(file: &str, var: &str) -> Result<PathBuf> {
    if let Some(named) = std::env::var_os(var).map(PathBuf::from) {
        if named.file_name() == Some(OsStr::new(file)) && named.is_file() {
            return Ok(named);
        }
    }
    user_model_dirs()
        .into_iter()
        .map(|dir| dir.join(file))
        .find(|path| path.is_file())
        .with_context(|| format!("{file} is not on disk: run 'just models' or name it in {var}"))
}

fn vad_models() -> Vec<PathBuf> {
    user_model_dirs()
        .into_iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| {
                    name.starts_with(VAD_MODEL_PREFIX)
                        && std::path::Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("bin"))
                })
        })
        .collect()
}

fn user_model_dirs() -> Vec<PathBuf> {
    let root = PathBuf::from(std::env::var_os("HOME").expect("HOME is set")).join(".voicepuncher");
    vec![root.join("models"), root]
}
