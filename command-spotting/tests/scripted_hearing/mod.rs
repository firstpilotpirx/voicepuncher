//! A speech recognition service for commands that knows the script of the stream: it finds
//! each clip in the stream and hears the words whose middle lies in it (or, as whisper does,
//! every word begun in it), after a set delay.
//! Lets a test run the engine's confirmations at any latency without a running service.

use std::ops::Range;
use std::time::Duration;

use voicepuncher_command_spotting::confirm::{Hear, Heard, TimedWord};
use voicepuncher_command_spotting::engine::SAMPLE_RATE;

/// Samples of a clip compared with the stream to find where the clip lies.
const PROBE_SAMPLES: usize = 64;

/// A word of the script and the samples of the stream it was said in.
pub struct ScriptedWord {
    pub text: String,
    pub said: Range<usize>,
}

/// Hears the words of `script` in each clip of `stream`, `delay` after it is asked.
pub struct ScriptedHearing {
    pub stream: Vec<f32>,
    pub script: Vec<ScriptedWord>,
    pub delay: Duration,
    /// Hears a word the clip's end cuts off whole, as whisper completes it ("start dicta…"
    /// heard as "start dictation"); else only the words whose middle lies in the clip.
    pub completes_cut_words: bool,
}

impl Hear for ScriptedHearing {
    fn hear(&self, samples: &[f32], _deadline: Duration) -> anyhow::Result<Heard> {
        std::thread::sleep(self.delay);
        Ok(self.heard_in(samples))
    }
}

impl ScriptedHearing {
    /// The words whose middle lies in `clip`, timed from its first sample; nothing for a clip
    /// that is not part of the stream (one heard again louder).
    fn heard_in(&self, clip: &[f32]) -> Heard {
        let Some(offset) = self.offset_of(clip) else {
            return Heard::default();
        };
        let clip_span = offset..offset + clip.len();
        let words: Vec<TimedWord> = self
            .script
            .iter()
            .filter(|word| clip_span.contains(&self.heard_at(word)))
            .map(|word| TimedWord {
                text: format!(" {}", word.text),
                start: seconds(word.said.start.max(offset) - offset),
                end: seconds(word.said.end.min(clip_span.end) - offset),
            })
            .collect();
        Heard {
            text: words.iter().map(|w| w.text.as_str()).collect(),
            words,
        }
    }

    /// The sample of `word` a clip must hold for the word to be heard.
    fn heard_at(&self, word: &ScriptedWord) -> usize {
        if self.completes_cut_words {
            word.said.start
        } else {
            (word.said.start + word.said.end) / 2
        }
    }

    /// Where `clip` starts in the stream, found by its first sound.
    fn offset_of(&self, clip: &[f32]) -> Option<usize> {
        let first_sound = clip.iter().position(|s| *s != 0.0)?;
        let probe = &clip[first_sound..clip.len().min(first_sound + PROBE_SAMPLES)];
        self.stream
            .windows(probe.len())
            .enumerate()
            .filter(|(at, window)| *at >= first_sound && *window == probe)
            .map(|(at, _)| at - first_sound)
            .find(|offset| self.stream.get(*offset..offset + clip.len()) == Some(clip))
    }
}

fn seconds(samples: usize) -> f32 {
    samples as f32 / SAMPLE_RATE as f32
}
