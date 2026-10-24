//! A home of the scenario's own for the catalog: its models, its `$PATH` folder and the
//! engine folders of a bundle next to the app.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use voicepuncher::adapters::secondary::localservices::catalog::Machine;

pub(super) const WHISPER: &str = "whisper-server";
pub(super) const COMMAND_SPOTTING: &str = "voicepuncher-command-spotting";
pub(super) const KWS_MODEL_DIR: &str = "sherpa-onnx-kws-zipformer";
pub(super) const COMMAND_MODEL_VAR: &str = "VOICEPUNCHER_COMMAND_WHISPER_MODEL";

/// A home with every model in `~/.voicepuncher/models` and every engine in one `$PATH` folder.
#[derive(Debug)]
pub(super) struct FakeHome {
    dir: tempfile::TempDir,
}

impl FakeHome {
    pub(super) fn complete() -> Result<Self> {
        let home = Self { dir: tempfile::tempdir()? };
        for file in [
            "ggml-large-v3-turbo-q5_0.bin",
            "ggml-small.bin",
            "silero_vad.onnx",
            "ggml-silero-v5.1.2.bin",
        ] {
            touch(&home.models().join(file))?;
        }
        std::fs::create_dir_all(home.models().join(KWS_MODEL_DIR))?;
        for engine in [WHISPER, COMMAND_SPOTTING] {
            touch(&home.bin().join(engine))?;
        }
        Ok(home)
    }

    pub(super) fn path(&self) -> &Path {
        self.dir.path()
    }

    pub(super) fn bin(&self) -> PathBuf {
        self.path().join("bin")
    }

    pub(super) fn models(&self) -> PathBuf {
        self.path().join(".voicepuncher/models")
    }

    pub(super) fn engines(&self) -> PathBuf {
        self.path().join("app/engines")
    }

    pub(super) fn machine(&self) -> Machine {
        Machine {
            home: self.path().to_path_buf(),
            env: HashMap::new(),
            path: vec![self.bin()],
            app_dir: Some(self.path().join("app")),
            cores: 12,
        }
    }
}

pub(super) fn touch(file: &Path) -> Result<()> {
    std::fs::create_dir_all(file.parent().context("a file in a folder")?)?;
    std::fs::write(file, b"")?;
    Ok(())
}
