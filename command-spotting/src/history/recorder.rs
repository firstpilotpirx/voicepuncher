//! Gathers what the command history keeps of a session: the phrases spotted, the hearings
//! asked and answered and the events reported, one entry per speech segment once it went out.
//!
//! Every time here is in seconds of the session's stream until an entry is cut, which
//! counts its times from the start of its audio.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::audio::SAMPLE_RATE;
use crate::confirm::RecentAudio;
use crate::conversions::saturating_usize;
use crate::protocol::{ServerEvent, Verdict};

use super::{outcome, Entry, Hearing, Outcome, Reported, SpottedPhrase};

/// How far past the end of its segment a spotted or reported phrase may end: as the
/// event orderer places phrases.
const SEGMENT_SLACK_SECS: f32 = 0.5;
/// How long past its end a segment that went out waits for late phrases and answers.
const CLOSE_AFTER_SECS: f32 = 1.0;
/// The audio kept before and after a segment's speech.
const AUDIO_MARGIN_SECS: f32 = 0.3;
/// The longest audio an entry keeps: its last seconds, which hold the command ending it.
const MAX_AUDIO_SECS: f32 = 10.0;
/// How long what no segment took is kept.
const KEPT_SECS: f32 = 30.0;

/// A speech segment that went out, waiting to be closed into an entry.
#[derive(Debug, Clone, Copy)]
struct Gone {
    start: f32,
    end: f32,
}

/// What one session's command history entries are made of.
pub struct Recorder {
    opened: SystemTime,
    source: String,
    entries: u64,
    spotted: Vec<SpottedPhrase>,
    /// Hearings asked and not answered yet, by job number.
    asked: HashMap<u64, Hearing>,
    heard: Vec<Hearing>,
    reported: Vec<Reported>,
    speech_start: Option<f32>,
    gone: Vec<Gone>,
}

impl Recorder {
    /// The recorder of connection `conn`, opened now.
    pub fn new(conn: u64) -> Self {
        Self {
            opened: SystemTime::now(),
            source: format!("{}-{conn}", std::process::id()),
            entries: 0,
            spotted: Vec::new(),
            asked: HashMap::new(),
            heard: Vec::new(),
            reported: Vec::new(),
            speech_start: None,
            gone: Vec::new(),
        }
    }

    /// The keyword spotter spotted `phrase` as `tokens` at `start..end`.
    pub fn spotted(&mut self, phrase: &str, tokens: &str, start: f32, end: f32) {
        let (phrase, tokens) = (phrase.to_owned(), tokens.to_owned());
        self.spotted
            .push(SpottedPhrase { phrase, tokens, start, end });
    }

    /// Job `n` asks the speech recognition service, as `kind`, about `start..end`.
    pub fn asked(&mut self, n: u64, kind: &str, start: f32, end: f32) {
        self.asked
            .insert(n, Hearing { kind: kind.to_owned(), start, end, text: None });
    }

    /// Job `n` was answered with `text`, or given up without one.
    pub fn answered(&mut self, n: u64, text: Option<&str>) {
        if let Some(mut hearing) = self.asked.remove(&n) {
            hearing.text = text.map(|t| t.trim().to_owned());
            self.heard.push(hearing);
        }
    }

    /// `events` left the session.
    pub fn went_out(&mut self, events: &[ServerEvent]) {
        for event in events {
            match event {
                ServerEvent::SpeechStart { t } => self.speech_start = Some(*t),
                ServerEvent::SpeechEnd { t } => {
                    let start = self.speech_start.take().unwrap_or(*t);
                    self.gone.push(Gone { start, end: *t });
                }
                ServerEvent::Phrase { phrase, start, end, timing } => {
                    self.report(vec![phrase.clone()], *start, *end, timing.is_some());
                }
                ServerEvent::Command { commands, start, end, timing, .. } => {
                    self.report(commands.clone(), *start, *end, timing.is_some());
                }
                ServerEvent::Heard { commands, verdict, .. } => self.verdict(commands, *verdict),
                _ => {}
            }
        }
    }

    fn report(&mut self, commands: Vec<String>, start: f32, end: f32, heard: bool) {
        self.reported
            .push(Reported { commands, start, end, heard, whisper: None });
    }

    /// The speech recognition service made `verdict` of `commands`, the last reported
    /// without confirmation.
    fn verdict(&mut self, commands: &[String], verdict: Verdict) {
        let unheard = self
            .reported
            .iter_mut()
            .rev()
            .find(|r| !r.heard && r.whisper.is_none() && r.commands == commands);
        if let Some(reported) = unheard {
            reported.whisper = Some(verdict);
        }
    }

    /// The entries of the segments due by `now`, or of every segment gone out with `all`.
    pub fn due(&mut self, now: f32, all: bool, audio: &RecentAudio) -> Vec<Entry> {
        let asked: Vec<Hearing> = self.asked.values().cloned().collect();
        let (due, waiting): (Vec<Gone>, Vec<Gone>) = self.gone.iter().partition(|g| {
            all || (g.end + CLOSE_AFTER_SECS <= now && !asked.iter().any(|h| overlaps(h, **g)))
        });
        self.gone = waiting;
        let entries = due
            .into_iter()
            .filter_map(|g| self.entry(g, audio))
            .collect();
        self.forget_before(now - KEPT_SECS);
        entries
    }

    fn entry(&mut self, gone: Gone, audio: &RecentAudio) -> Option<Entry> {
        let holds = |end: f32| end >= gone.start && end <= gone.end + SEGMENT_SLACK_SECS;
        let spotted: Vec<SpottedPhrase> = take(&mut self.spotted, |s| holds(s.end));
        let reported: Vec<Reported> = take(&mut self.reported, |r| holds(r.end));
        let mut hearings: Vec<Hearing> = take(&mut self.heard, |h| overlaps(h, gone));
        hearings.extend(self.asked.values().filter(|h| overlaps(h, gone)).cloned());
        let outcome = outcome_of(&spotted, &reported)?;
        let (from, to) = audio_span(gone, &spotted, &reported);
        self.entries += 1;
        Some(Entry {
            started_at: self.opened + Duration::from_secs_f32(from),
            source: format!("{}-{}", self.source, self.entries),
            audio: audio.between(sample_at(from), sample_at(to)),
            transcript: transcript(&hearings),
            spotted: shifted(spotted, from),
            hearings: shifted(hearings, from),
            reported: shifted(reported, from),
            outcome,
        })
    }

    fn forget_before(&mut self, before: f32) {
        self.spotted.retain(|s| s.end >= before);
        self.reported.retain(|r| r.end >= before);
        self.heard.retain(|h| h.end >= before);
    }
}

/// The outcome of the phrases the keyword spotter spotted against the commands the speech
/// recognition service heard, before or after they went out; none when neither has any.
fn outcome_of(spotted: &[SpottedPhrase], reported: &[Reported]) -> Option<Outcome> {
    let spotted: Vec<&str> = spotted.iter().map(|s| s.phrase.as_str()).collect();
    let heard: Vec<&str> = reported
        .iter()
        .filter(|r| r.heard || r.whisper == Some(Verdict::Agreed))
        .flat_map(|r| r.commands.iter().map(String::as_str))
        .collect();
    outcome(&spotted, &heard)
}

/// The audio an entry keeps: from the earliest phrase or the segment's start to the segment's
/// end, with a margin on each side, at most its last seconds.
fn audio_span(gone: Gone, spotted: &[SpottedPhrase], reported: &[Reported]) -> (f32, f32) {
    let to = gone.end + AUDIO_MARGIN_SECS;
    let earliest = spotted
        .iter()
        .map(|s| s.start)
        .chain(reported.iter().map(|r| r.start));
    let from = (earliest.fold(gone.start, f32::min) - AUDIO_MARGIN_SECS)
        .max(to - MAX_AUDIO_SECS)
        .max(0.0);
    (from, to)
}

/// Something placed in the stream, which an entry places in its own audio.
trait Placed {
    fn moved_back(self, by: f32) -> Self;
}

impl Placed for SpottedPhrase {
    fn moved_back(self, by: f32) -> Self {
        Self { start: self.start - by, end: self.end - by, ..self }
    }
}

impl Placed for Hearing {
    fn moved_back(self, by: f32) -> Self {
        Self { start: self.start - by, end: self.end - by, ..self }
    }
}

impl Placed for Reported {
    fn moved_back(self, by: f32) -> Self {
        Self { start: self.start - by, end: self.end - by, ..self }
    }
}

/// `items` timed from `from` in the stream.
fn shifted<T: Placed>(items: Vec<T>, from: f32) -> Vec<T> {
    items
        .into_iter()
        .map(|item| item.moved_back(from))
        .collect()
}

fn overlaps(hearing: &Hearing, gone: Gone) -> bool {
    hearing.start < gone.end && hearing.end > gone.start
}

/// What the segment's own hearing heard, else the last hearing that heard anything.
fn transcript(hearings: &[Hearing]) -> String {
    let segment = hearings
        .iter()
        .rev()
        .find(|h| h.kind == "segment" && h.text.is_some());
    segment
        .or_else(|| hearings.iter().rev().find(|h| h.text.is_some()))
        .and_then(|h| h.text.clone())
        .unwrap_or_default()
}

/// The items of `items` that `taken` picks, in order; the rest stay.
fn take<T>(items: &mut Vec<T>, taken: impl Fn(&T) -> bool) -> Vec<T> {
    let (picked, left) = std::mem::take(items)
        .into_iter()
        .partition(|item| taken(item));
    *items = left;
    picked
}

fn sample_at(secs: f32) -> usize {
    saturating_usize(secs * SAMPLE_RATE as f32)
}

#[cfg(test)]
mod tests;
