//! The user's recorded voice sessions (`VOICEPUNCHER_VOICE_SAMPLES`), each a recording beside
//! a folder that labels its stretches.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_test_steps::required_env;

/// A recorded session: the whole recording and the labels of its stretches.
pub struct Session<L> {
    pub name: String,
    pub audio: Vec<f32>,
    pub labels: Vec<L>,
}

/// The command spotting service's URL and every session whose folder holds `labels_file`, by
/// name.
pub fn sessions<L: DeserializeOwned>(labels_file: &str) -> (String, Vec<Session<L>>) {
    let url = required_env("VOICEPUNCHER_COMMAND_SPOTTING_URL");
    let root = required_env("VOICEPUNCHER_VOICE_SAMPLES");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join(labels_file).is_file() && p.with_extension("wav").is_file())
        .collect();
    dirs.sort();
    (url, dirs.iter().map(|dir| load(dir, labels_file)).collect())
}

fn load<L: DeserializeOwned>(dir: &Path, labels_file: &str) -> Session<L> {
    let labels = std::fs::read_to_string(dir.join(labels_file))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let audio = read_wav(&dir.with_extension("wav"))
        .unwrap()
        .into_iter()
        .map(|s| f32::from(s) / f32::from(i16::MAX))
        .collect();
    Session { name: dir.file_name().unwrap().to_string_lossy().into_owned(), audio, labels }
}
