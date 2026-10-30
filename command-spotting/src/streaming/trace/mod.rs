//! What one streaming utterance went through: each recognizer's text as it changed, how long
//! its decoding took, and each report with its delay after the last word it covers.
//!
//! Times are seconds of the connection's stream until [`Trace::shifted`] counts them from
//! the start of the entry's audio.

use std::fmt::Write;
use std::time::Duration;

use serde::Serialize;

use super::decide::Finding;
use crate::conversions::saturating_usize;

/// A recognizer's text from `at` on.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextAt {
    pub at: f32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecognizerTrace {
    pub name: String,
    pub texts: Vec<TextAt>,
    /// All its decoding of the utterance, and its slowest frame.
    pub decode_ms: u64,
    pub slowest_frame_ms: u64,
}

/// Commands a recognizer reported at `at`, said at `start..end`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportTrace {
    pub recognizer: String,
    pub commands: Vec<String>,
    pub start: f32,
    pub end: f32,
    pub at: f32,
    /// Audio after the last word of the commands before they went out.
    pub delay_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Trace {
    /// Where its audio starts: the replay before the onset.
    pub start: f32,
    /// Where the gate heard speech.
    pub onset: f32,
    pub speech_end: f32,
    pub closed: f32,
    pub recognizers: Vec<RecognizerTrace>,
    pub reports: Vec<ReportTrace>,
}

impl Trace {
    pub fn new(start: f32, onset: f32, names: &[&str]) -> Self {
        let recognizer = |name: &&str| RecognizerTrace {
            name: (*name).to_owned(),
            texts: Vec::new(),
            decode_ms: 0,
            slowest_frame_ms: 0,
        };
        Self {
            start,
            onset,
            speech_end: onset,
            closed: onset,
            recognizers: names.iter().map(recognizer).collect(),
            reports: Vec::new(),
        }
    }

    /// Recognizer `r` holds `text` at `at`; kept only when it changed.
    pub fn heard(&mut self, r: usize, at: f32, text: &str) {
        let texts = &mut self.recognizers[r].texts;
        if texts
            .last()
            .map_or(!text.is_empty(), |last| last.text != text)
        {
            texts.push(TextAt { at, text: text.to_owned() });
        }
    }

    /// Recognizer `r` decoded one frame in `took`.
    pub fn decoded(&mut self, r: usize, took: Duration) {
        let ms = u64::try_from(took.as_millis()).unwrap_or(u64::MAX);
        let recognizer = &mut self.recognizers[r];
        recognizer.decode_ms += ms;
        recognizer.slowest_frame_ms = recognizer.slowest_frame_ms.max(ms);
    }

    pub fn reported(&mut self, r: usize, finding: &Finding, at: f32) {
        self.reports.push(ReportTrace {
            recognizer: self.recognizers[r].name.clone(),
            commands: finding.commands.clone(),
            start: finding.start,
            end: finding.end,
            at,
            delay_ms: millis(at - finding.end),
        });
    }

    /// The last text recognizer `r` held.
    pub fn final_text(&self, r: usize) -> &str {
        self.recognizers[r]
            .texts
            .last()
            .map_or("", |t| t.text.as_str())
    }

    /// One log line of the utterance: its bounds, each recognizer's text and decoding, each
    /// report and its delay.
    pub fn log_line(&self) -> String {
        let mut line = format!(
            "utterance {:.2}–{:.2}s closed {:.2}s",
            self.onset, self.speech_end, self.closed
        );
        for (r, recognizer) in self.recognizers.iter().enumerate() {
            // WHY: writing to a String cannot fail.
            let _ = write!(
                line,
                " | {} {:?} {}ms/{}ms",
                recognizer.name,
                self.final_text(r),
                recognizer.decode_ms,
                recognizer.slowest_frame_ms
            );
        }
        for report in &self.reports {
            // WHY: writing to a String cannot fail.
            let _ = write!(
                line,
                " | {} by {} at {:.2}s +{}ms",
                report.commands.join(", "),
                report.recognizer,
                report.at,
                report.delay_ms
            );
        }
        line
    }

    /// The trace with every time counted from the start of its audio.
    pub fn shifted(&self) -> Self {
        let from = self.start;
        let mut trace = self.clone();
        trace.start = 0.0;
        trace.onset -= from;
        trace.speech_end -= from;
        trace.closed -= from;
        for text in trace
            .recognizers
            .iter_mut()
            .flat_map(|r| r.texts.iter_mut())
        {
            text.at -= from;
        }
        for report in &mut trace.reports {
            report.start -= from;
            report.end -= from;
            report.at -= from;
        }
        trace
    }
}

fn millis(secs: f32) -> u64 {
    saturating_usize((secs.max(0.0) * 1_000.0).round()) as u64
}

#[cfg(test)]
pub(crate) mod tests;
