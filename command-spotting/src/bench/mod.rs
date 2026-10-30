//! The streaming bench: the user's labelled clips streamed through the streaming session, in
//! real time or as fast as it decodes, scored for the commands heard and how late they came.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

mod run;
mod summary;
pub use run::{run_clip, Pace};
pub use summary::{Outcome, Summary};

/// One labelled clip of the bench's dataset.
#[derive(Debug, Clone)]
pub struct Clip {
    pub id: String,
    pub audio: Vec<f32>,
    /// The command words said, in order (`start`, `enter`, …); empty when none was said.
    pub label: Vec<String>,
    /// Where, in seconds of the clip, the words of its only command end.
    pub word_end: Option<f32>,
}

/// A line of `dataset.jsonl`, as the research bench of `streaming-command-recognition` wrote it.
#[derive(Debug, Deserialize)]
struct Line {
    id: String,
    path: String,
    source: String,
    #[serde(default)]
    label: Vec<String>,
    #[serde(default)]
    cut: Option<(f32, f32)>,
    #[serde(default)]
    word_end: Option<f32>,
    /// History labels count only where two Whisper hearings agreed.
    #[serde(default)]
    confident: Option<bool>,
}

/// The clips of the dataset at `path` whose label is trusted.
pub fn load_dataset(path: &Path) -> Result<Vec<Clip>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the dataset {}", path.display()))?;
    let mut clips = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let line: Line = serde_json::from_str(line).context("a dataset line")?;
        if line.source == "history" && line.confident != Some(true) {
            continue;
        }
        let audio = read_wav(Path::new(&line.path))?;
        let audio = line
            .cut
            .map_or(audio.clone(), |(from, to)| cut(&audio, from, to));
        let word_end = (line.label.len() == 1).then_some(line.word_end).flatten();
        clips.push(Clip { id: line.id, audio, label: line.label, word_end });
    }
    Ok(clips)
}

/// The command word a reported phrase counts as in a label: `press enter` → `enter`.
pub fn label_word(phrase: &str) -> &str {
    match phrase {
        "press enter" => "enter",
        other => other.split_whitespace().next().unwrap_or(other),
    }
}

fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    reader
        .samples::<i16>()
        .map(|s| s.map(|s| f32::from(s) / f32::from(i16::MAX)))
        .collect::<Result<_, _>>()
        .with_context(|| format!("cannot read {}", path.display()))
}

fn cut(audio: &[f32], from: f32, to: f32) -> Vec<f32> {
    let at = |secs: f32| crate::conversions::saturating_usize(secs * 16_000.0).min(audio.len());
    audio[at(from)..at(to)].to_vec()
}

#[cfg(test)]
mod tests;
