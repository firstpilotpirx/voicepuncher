use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::config::Config;
use crate::core::application::{ProbeAnswer, ServiceProbe};
use crate::core::domain::{ServiceKind, ServiceSpec};

mod copies;
mod engines;
mod models;

/// Answers Ready on the given ports, Closed elsewhere.
struct Answering(Vec<u16>);

impl ServiceProbe for Answering {
    fn answer(&self, _kind: ServiceKind, port: u16) -> ProbeAnswer {
        if self.0.contains(&port) {
            ProbeAnswer::Ready
        } else {
            ProbeAnswer::Closed
        }
    }
}

const NOTHING_RUNS: Answering = Answering(Vec::new());

/// A home with every model in `~/.voicepuncher/models` and every engine in one `$PATH` folder.
struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    fn complete() -> Self {
        let home = Self::empty();
        let models = home.path().join(".voicepuncher/models");
        for file in [
            DICTATION_MODEL,
            COMMAND_MODEL,
            KWS_VAD_MODEL,
            "ggml-silero-v5.1.2.bin",
        ] {
            touch(&models.join(file));
        }
        fs::create_dir_all(models.join("sherpa-onnx-kws-zipformer")).unwrap();
        for engine in [WHISPER, COMMAND_SPOTTING] {
            touch(&home.path().join("bin").join(engine));
        }
        home
    }

    fn empty() -> Self {
        Self { dir: tempfile::tempdir().unwrap() }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn machine(&self) -> Machine {
        Machine {
            home: self.path().to_path_buf(),
            env: HashMap::new(),
            path: vec![self.path().join("bin")],
            app_dir: Some(self.path().join("app")),
            cores: 12,
        }
    }

    fn models(&self) -> PathBuf {
        self.path().join(".voicepuncher/models")
    }
}

fn kinds_and_ports(specs: &[ServiceSpec]) -> Vec<(ServiceKind, u16)> {
    specs.iter().map(|spec| (spec.kind, spec.port)).collect()
}

mod adoption;
mod command_lines;

const ALL: Needs = Needs { voice_control: true };

/// Writes an empty `file`, with the folders it needs.
fn touch(file: &Path) {
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, b"").unwrap();
}
