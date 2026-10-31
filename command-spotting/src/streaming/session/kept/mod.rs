//! The command history entry of a closed streaming utterance.

use std::time::{Duration, SystemTime};

use crate::history::{outcome, Entry, Outcome, Reported, SpottedPhrase};

use super::super::decide::Finding;
use super::super::trace::{ReportTrace, Trace};

/// When the connection opened and the name of the entry: what the history keeps it by.
pub struct Kept {
    pub opened: SystemTime,
    pub source: String,
}

/// The entry of the utterance `trace` followed, its times counted from its audio's start;
/// its outcome `none` when no final text holds a command (`heard`, by recognizer) and nothing
/// was reported, so a missed command can still be found.
pub fn entry(trace: &Trace, heard: &[(usize, Finding)], audio: Vec<f32>, kept: Kept) -> Entry {
    let spotted: Vec<SpottedPhrase> = heard
        .iter()
        .flat_map(|(r, found)| spotted(trace, *r, found))
        .collect();
    let reported: Vec<Reported> = trace
        .reports
        .iter()
        .map(|r| as_reported(trace, r))
        .collect();
    let spotted_names: Vec<&str> = spotted.iter().map(|s| s.phrase.as_str()).collect();
    let reported_names: Vec<&str> = reported
        .iter()
        .flat_map(|r| r.commands.iter().map(String::as_str))
        .collect();
    let outcome = outcome(&spotted_names, &reported_names).unwrap_or(Outcome::None);
    Entry {
        started_at: kept.opened + Duration::from_secs_f32(trace.start.max(0.0)),
        source: kept.source,
        audio,
        spotted,
        hearings: Vec::new(),
        transcript: transcript(trace),
        reported,
        outcome,
        streaming: Some(trace.shifted()),
    }
}

fn spotted(trace: &Trace, r: usize, found: &Finding) -> Vec<SpottedPhrase> {
    let tokens = format!("{}: {}", trace.recognizers[r].name, trace.final_text(r));
    found
        .commands
        .iter()
        .map(|phrase| SpottedPhrase {
            phrase: phrase.clone(),
            tokens: tokens.clone(),
            start: found.start - trace.start,
            end: found.end - trace.start,
        })
        .collect()
}

fn as_reported(trace: &Trace, report: &ReportTrace) -> Reported {
    Reported {
        commands: report.commands.clone(),
        start: report.start - trace.start,
        end: report.end - trace.start,
        heard: false,
        whisper: None,
    }
}

/// Every recognizer's final text, by name.
fn transcript(trace: &Trace) -> String {
    let texts: Vec<String> = (0..trace.recognizers.len())
        .map(|r| format!("{}: {}", trace.recognizers[r].name, trace.final_text(r)))
        .collect();
    texts.join(" | ")
}

#[cfg(test)]
mod tests;
