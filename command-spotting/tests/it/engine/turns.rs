//! Hearings that answer in turn, from the clip or late, and feeding audio in real time.

use super::*;
use voicepuncher_command_spotting::conversions::saturating_usize;

pub(super) const DICTATING_PHRASES: [&str; 4] = [
    "stop dictation",
    "cancel dictation",
    "restart dictation",
    "press enter",
];

/// Hears each answer in turn, one per clip, and nothing once they run out; counts the clips.
pub(super) struct HearsInTurn {
    /// The answer to the segment's clip, which starts with the pre-roll.
    first: Mutex<Option<&'static str>>,
    /// The answers to the steady clips, in turn: a second hearing may be asked beside the
    /// first one, so the order of the requests says nothing.
    again: Mutex<std::collections::VecDeque<&'static str>>,
    clips: Mutex<usize>,
}

/// The steady clip starts 0.25 s before its first sound; a segment's clip starts with the
/// pre-roll of 0.8 s before the detected speech.
pub(super) const STEADY_LEAD_MAX_SECS: f32 = 0.45;

impl HearsInTurn {
    pub(crate) fn new(answers: &[&'static str]) -> Arc<Self> {
        let (first, again) = answers.split_first().expect("an answer");
        Arc::new(Self {
            first: Mutex::new(Some(*first)),
            again: Mutex::new(again.iter().copied().collect()),
            clips: Mutex::new(0),
        })
    }

    pub(crate) fn clips(&self) -> usize {
        *self.clips.lock().unwrap()
    }
}

impl Hear for HearsInTurn {
    fn hear(&self, clip: &[f32], _: Duration) -> anyhow::Result<Heard> {
        *self.clips.lock().unwrap() += 1;
        let onset = clip
            .iter()
            .position(|s| s.abs() > SILENCE_LEVEL)
            .unwrap_or(0);
        let steady = (onset as f32) < STEADY_LEAD_MAX_SECS * SAMPLE_RATE as f32;
        let text = if steady {
            self.again.lock().unwrap().pop_front()
        } else {
            self.first.lock().unwrap().take()
        }
        .unwrap_or("");
        let words: Vec<&str> = text.split_whitespace().collect();
        Ok(Heard { text: text.to_owned(), words: timed(&words) })
    }
}

/// The events of `words` said on its own while `phrases` are selected, heard by `hear`.
pub(super) fn heard_alone(
    hear: Arc<HearsInTurn>,
    phrases: &[&str],
    words: &str,
) -> Vec<ServerEvent> {
    let engine = own_engine(None).hearing(hear);
    let mut session = listening(&engine, phrases);
    fed(&mut session, &spoken(words))
}

/// Hears exactly the words of the stream that a clip holds, with their times in the clip: a
/// word the clip's end cuts off is heard as "style", as the speech recognition service heard
/// the user's cut-off "start". Records the words a clip cut off.
pub(super) struct HearsTheWordsInTheClip {
    pub(crate) stream: Vec<f32>,
    /// Each word and the stream samples it spans.
    pub(crate) words: Vec<(&'static str, std::ops::Range<usize>)>,
    pub(crate) cut_off: Mutex<Vec<&'static str>>,
}

/// Loud enough to be the start of a word, not the room.
pub(super) const SOUND_LEVEL: f32 = 0.01;

/// Samples that place a clip in the stream.
pub(super) const MARK_LEN: usize = 64;

impl HearsTheWordsInTheClip {
    /// `words` said one after another, `gap_secs` of silence between them, after a pause.
    pub(crate) fn said(words: &[&'static str], gap_secs: f32) -> Arc<Self> {
        let pairs: Vec<_> = words.iter().map(|w| (*w, *w)).collect();
        Self::said_as(&pairs, gap_secs)
    }

    /// Each word heard as its first part while the stream says its second, as `said` lays
    /// them out: a word the keyword spotter does not know, heard as a command.
    pub(crate) fn said_as(words: &[(&'static str, &'static str)], gap_secs: f32) -> Arc<Self> {
        let gap = vec![0.0f32; saturating_usize(gap_secs * SAMPLE_RATE as f32)];
        let mut stream = vec![0.0f32; saturating_usize(PAUSE_SECS * SAMPLE_RATE as f32)];
        let mut spans = Vec::new();
        for (n, (word, spoken)) in words.iter().enumerate() {
            if n > 0 {
                stream.extend(&gap);
            }
            let start = stream.len();
            stream.extend(without_silence(&common::spoken_by(None, spoken)));
            spans.push((*word, start..stream.len()));
        }
        stream.extend(vec![
            0.0f32;
            saturating_usize(PAUSE_SECS * SAMPLE_RATE as f32)
        ]);
        Arc::new(Self { stream, words: spans, cut_off: Mutex::new(Vec::new()) })
    }

    /// Where `clip` starts in the stream, found by its first sound.
    fn origin(&self, clip: &[f32]) -> Option<usize> {
        let at = clip.iter().position(|s| s.abs() > SOUND_LEVEL)?;
        let mark = clip.get(at..at + MARK_LEN)?;
        let found = self.stream.windows(MARK_LEN).position(|w| w == mark)?;
        found.checked_sub(at)
    }
}

/// `audio` from its first sound to its last: `say` puts silence around every word.
pub(super) fn without_silence(audio: &[f32]) -> &[f32] {
    let first = audio
        .iter()
        .position(|s| s.abs() > SOUND_LEVEL)
        .unwrap_or(0);
    let last = audio
        .iter()
        .rposition(|s| s.abs() > SOUND_LEVEL)
        .unwrap_or(0);
    &audio[first..=last.max(first)]
}

impl Hear for HearsTheWordsInTheClip {
    fn hear(&self, clip: &[f32], _: Duration) -> anyhow::Result<Heard> {
        let Some(origin) = self.origin(clip) else {
            return Ok(Heard::default());
        };
        let end = origin + clip.len();
        let secs = |at: usize| (at - origin) as f32 / SAMPLE_RATE as f32;
        let mut words = Vec::new();
        for (word, span) in &self.words {
            if span.start < origin || span.start >= end {
                continue;
            }
            let cut = span.end > end;
            if cut {
                self.cut_off.lock().unwrap().push(word);
            }
            words.push(voicepuncher_command_spotting::confirm::TimedWord {
                text: format!(" {}", if cut { "style" } else { word }),
                start: secs(span.start),
                end: secs(span.end.min(end)),
            });
        }
        Ok(Heard { text: words.iter().map(|w| w.text.as_str()).collect(), words })
    }
}

/// Every event for `audio` fed frame by frame, each confirmation answered before the next
/// frame: a service that answers at once, so every check of the running utterance is asked.
pub(super) fn fed_answering_at_once(session: &mut Session<'_>, audio: &[f32]) -> Vec<ServerEvent> {
    let mut events = Vec::new();
    for frame in audio.chunks(FRAME_SAMPLES) {
        events.extend(session.accept(frame));
        events.extend(session.settle());
    }
    events.extend(session.settle());
    events
}

/// How long after the end of its words a start followed at once by speech may be reported.
pub(super) const START_REPORTED_WITHIN_SECS: f32 = 0.6;

/// Before the search of the utterance's start 1.0 s in could report it: only the spotted check.
pub(super) const SPOTTED_CHECK_WITHIN_SECS: f32 = 0.5;

pub(super) const STANDBY_PHRASES: [&str; 3] = ["start dictation", "press enter", "clear all text"];

/// Each command reported in `audio` fed frame by frame, every confirmation answered before
/// the next frame, with the stream position at which it was reported.
pub(super) fn reported_at(session: &mut Session<'_>, audio: &[f32]) -> Vec<(Vec<String>, usize)> {
    let mut reported = Vec::new();
    let mut fed_so_far = 0;
    for frame in audio.chunks(FRAME_SAMPLES) {
        let mut events = session.accept(frame);
        events.extend(session.settle());
        fed_so_far += frame.len();
        for event in events {
            if let ServerEvent::Command { commands, .. } = event {
                reported.push((commands, fed_so_far));
            }
        }
    }
    reported
}

/// What a trigger-only keyword would be named if it ever went out as a phrase.
pub(super) fn trigger_phrases(events: &[ServerEvent]) -> Vec<String> {
    spotted(events)
        .into_iter()
        .filter(|phrase| !ALL_PHRASES.contains(&phrase.as_str()))
        .collect()
}

/// How late after the end of the words at `command_end` the first command of `phrases` was
/// reported for `hear`'s stream, every confirmation answered at once; every event too.
pub(super) fn report_delay(
    hear: &Arc<HearsTheWordsInTheClip>,
    phrases: &[&str],
    command_end: usize,
) -> (Option<f32>, Vec<ServerEvent>) {
    let engine = own_engine(None).hearing(hear.clone());
    let mut session = listening(&engine, phrases);
    let mut events = Vec::new();
    let mut delay = None;
    let mut fed_so_far = 0;
    for frame in hear.stream.chunks(FRAME_SAMPLES) {
        let mut new = session.accept(frame);
        new.extend(session.settle());
        fed_so_far += frame.len();
        if delay.is_none() && new.iter().any(|e| matches!(e, ServerEvent::Command { .. })) {
            delay = Some((fed_so_far as f32 - command_end as f32) / SAMPLE_RATE as f32);
        }
        events.extend(new);
    }
    (delay, events)
}

/// Answers as `words` does, the first clip only after `SLOW_FIRST_ANSWER`: a check of the
/// running utterance answered after the utterance's own segment.
pub(super) struct SlowFirstClip {
    pub(crate) words: Arc<HearsTheWordsInTheClip>,
    pub(crate) clips: Mutex<usize>,
}

pub(super) const SLOW_FIRST_ANSWER: Duration = Duration::from_millis(1_500);

impl Hear for SlowFirstClip {
    fn hear(&self, clip: &[f32], deadline: Duration) -> anyhow::Result<Heard> {
        let first = {
            let mut clips = self.clips.lock().unwrap();
            *clips += 1;
            *clips == 1
        };
        if first {
            std::thread::sleep(SLOW_FIRST_ANSWER);
        }
        self.words.hear(clip, deadline)
    }
}

/// Every event for `audio` fed in real time, frame by frame, its confirmations included.
pub(super) fn fed_in_real_time(session: &mut Session<'_>, audio: &[f32]) -> Vec<ServerEvent> {
    let frame_time = Duration::from_secs_f32(FRAME_SAMPLES as f32 / SAMPLE_RATE as f32);
    let mut events = Vec::new();
    for frame in audio.chunks(FRAME_SAMPLES) {
        events.extend(session.accept(frame));
        std::thread::sleep(frame_time);
        events.extend(session.poll());
    }
    events.extend(session.settle());
    events
}
