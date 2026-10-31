//! The user's recorded sessions (`~/.voicepuncher/voice-samples/session-N.wav`) as bench clips:
//! each labelled row of a session cut out with the silence around it, listening for the
//! commands of the state it was said in.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use voicepuncher_voice_commands::{DICTATING, STANDBY};

use super::{cut, label_word, read_wav, Clip};

/// Audio kept before a row's words, unless the row before ends sooner.
const LEAD_SECS: f32 = 0.6;
/// Audio kept after a row's words, unless the row after starts sooner.
const TRAIL_SECS: f32 = 1.0;

/// A labelled row of a session: `forms.jsonl` (`commands`) or session 1's `manifest.jsonl`
/// (`command`), in seconds of the session's recording.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Row {
    pub start: f32,
    pub end: f32,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    command: Option<String>,
    /// Too quiet or unclear to count.
    #[serde(default)]
    pub unclear: bool,
}

impl Row {
    /// The commands the row says, in order.
    fn said(&self) -> Vec<String> {
        self.command.iter().chain(&self.commands).cloned().collect()
    }
}

/// Where a row is cut from its session and what it says.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub from: f32,
    pub to: f32,
    pub label: Vec<String>,
    pub listening: Vec<String>,
    /// Where, in seconds of the clip, the words of its only command end.
    pub word_end: Option<f32>,
}

/// The spans of a session's `rows`, sorted by time; unclear rows are left out.
pub fn spans(rows: &[Row]) -> Vec<(usize, Span)> {
    let mut sorted: Vec<(usize, &Row)> = rows.iter().enumerate().collect();
    sorted.sort_by(|a, b| a.1.start.total_cmp(&b.1.start));
    (0..sorted.len())
        .filter(|&i| !sorted[i].1.unclear)
        .map(|i| {
            let (n, row) = sorted[i];
            let before = i.checked_sub(1).map_or(0.0, |p| sorted[p].1.end);
            let after = sorted.get(i + 1).map_or(f32::MAX, |(_, r)| r.start);
            let from = (row.start - LEAD_SECS).max(before).max(0.0);
            let label: Vec<String> = row
                .said()
                .iter()
                .map(|c| label_word(c).to_owned())
                .collect();
            let word_end = (label.len() == 1).then_some(row.end - from);
            let span = Span {
                from,
                to: (row.end + TRAIL_SECS).min(after),
                label,
                listening: listening(row.state.as_deref(), &row.said()),
                word_end,
            };
            (n, span)
        })
        .collect()
}

/// The commands an app listens for in `state`; without one, those of the commands said.
fn listening(state: Option<&str>, said: &[String]) -> Vec<String> {
    let phrases = match state {
        Some("standby") => STANDBY,
        Some(_) => DICTATING,
        None if said.iter().any(|c| STANDBY.contains(&c.as_str())) => STANDBY,
        None => DICTATING,
    };
    phrases.iter().map(|p| (*p).to_owned()).collect()
}

/// Every labelled clip of the sessions in `voice_samples`.
pub fn recorded_clips(voice_samples: &Path) -> Result<Vec<Clip>> {
    let mut clips = Vec::new();
    for n in 1.. {
        let session = format!("session-{n}");
        let wav = voice_samples.join(format!("{session}.wav"));
        if !wav.exists() {
            break;
        }
        let audio = read_wav(&wav)?;
        let rows = rows(&voice_samples.join(&session))?;
        clips.extend(spans(&rows).into_iter().map(|(i, span)| Clip {
            id: format!("{session}/{i}"),
            audio: cut(&audio, span.from, span.to),
            label: span.label,
            listening: span.listening,
            word_end: span.word_end,
        }));
    }
    Ok(clips)
}

/// The rows of a session folder: `forms.jsonl`, else `manifest.jsonl`.
fn rows(folder: &Path) -> Result<Vec<Row>> {
    let forms = folder.join("forms.jsonl");
    let file = if forms.exists() {
        forms
    } else {
        folder.join("manifest.jsonl")
    };
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("cannot read {}", file.display()))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).with_context(|| format!("a row of {}", file.display())))
        .collect()
}
