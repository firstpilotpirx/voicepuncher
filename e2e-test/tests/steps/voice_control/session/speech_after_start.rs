//! Speech said right after a voice command that starts a dictation, end to end: the speech
//! streams in real time, as from a microphone, so the command spotting service reports the
//! command while the next words are already being said.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};
use serde::Deserialize;

use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::{
    from_first_sound, silence, spoken_by, without_lead_pause, without_trailing_silence,
};

use super::{
    command_audio, env, room_noise, session_of, to_f32, words, CommandVoice, Event, Expected,
    HandsFreeSession, ENGLISH_VOICE, SAMPLE_RATE_HZ,
};
use crate::E2eWorld;

/// The dictation said right after the command; its first words are what a late start loses.
pub(super) const SENTENCE: &str = "Coffee machine on the third floor is broken";
/// Words of the commands that start a dictation: none may be pasted.
const START_WORDS: [&str; 3] = ["start", "restart", "dictation"];
/// Said before a restart: none of it may be pasted.
pub(super) const BEFORE_RESTART: &str = "Weather is sunny in Berlin today";
/// The silence `say` speech starts and ends with.
const SAY_PAUSE_SECS: f32 = 1.0;
/// 20 ms per message: a real microphone delivers audio about this often.
const REAL_TIME_FRAME_SAMPLES: usize = 320;
/// Recording kept before and after a labelled stretch, so its first and last sounds are
/// whole: the release of a final "t" comes up to 0.15 s after the labelled end.
const LEAD_MARGIN_SECS: f32 = 0.3;
const TAIL_MARGIN_SECS: f32 = 0.2;
/// A short breath between a sentence and a command, from the last sound to the next: longer
/// than the 0.3 s that ends an utterance, shorter than a deliberate pause.
const BREATH_SECS: f32 = 0.35;
/// The share of a recording's loudest 10 ms that still counts as its sound.
const LAST_SOUND_FRACTION: f32 = 0.03;
/// 10 ms at 16 kHz.
const LOUDNESS_FRAME_SAMPLES: usize = 160;
/// The user's labelled recording sessions, searched in order for a way of saying a command.
const RECORDED_SESSIONS: [&str; 2] = ["session-2", "session-5"];

#[derive(Deserialize)]
pub(super) struct Form {
    pub(super) start: f32,
    end: f32,
    pub(super) said: String,
    #[serde(default)]
    pub(super) state: String,
    #[serde(default)]
    pub(super) commands: Vec<String>,
}

fn samples_in(secs: f32) -> usize {
    (secs * SAMPLE_RATE_HZ) as usize
}

/// A recording cut after its last 10 ms louder than 3 % of its loudest: the room's silence
/// after the word would lengthen any pause built after it.
pub(in crate::steps::voice_control) fn until_last_sound(mut audio: Vec<f32>) -> Vec<f32> {
    let loudness: Vec<f32> = audio
        .chunks(LOUDNESS_FRAME_SAMPLES)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .collect();
    let loudest = loudness.iter().copied().fold(0.0, f32::max);
    let last = loudness
        .iter()
        .rposition(|l| *l > loudest * LAST_SOUND_FRACTION);
    audio.truncate(last.map_or(0, |n| ((n + 1) * LOUDNESS_FRAME_SAMPLES).min(audio.len())));
    audio
}

/// The user's quiet "restart" that the speech recognition service hears as "Let's start."
/// unless it is heard again: labelled at 27.23 s in `session-2`.
const QUIET_RESTART: (&str, f32) = ("session-2", 27.23);

/// The labelled forms of a recording session.
pub(super) fn forms_of(root: &Path, session: &str) -> Result<Vec<Form>> {
    let labels = std::fs::read_to_string(root.join(session).join("forms.jsonl"))?;
    Ok(labels
        .lines()
        .map(serde_json::from_str::<Form>)
        .collect::<serde_json::Result<Vec<_>>>()?)
}

/// `form` cut from its recording session with the margins that keep its first and last sounds.
pub(super) fn cut(root: &Path, session: &str, form: &Form) -> Result<Vec<f32>> {
    let audio = to_f32(read_wav(&root.join(format!("{session}.wav")))?);
    let from = samples_in(form.start - LEAD_MARGIN_SECS);
    let to = samples_in(form.end + TAIL_MARGIN_SECS).min(audio.len());
    Ok(audio[from..to].to_vec())
}

/// The user's first recorded `said`, cut from their labelled recording sessions.
pub(in crate::steps::voice_control) fn recorded_form(said: &str) -> Result<Vec<f32>> {
    let root = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?);
    for session in RECORDED_SESSIONS {
        if let Some(form) = forms_of(&root, session)?
            .into_iter()
            .find(|form| form.said == said)
        {
            return cut(&root, session, &form);
        }
    }
    bail!("no recording of {said:?} in {RECORDED_SESSIONS:?}")
}

/// The user's quiet recorded "restart", picked by its label.
pub(in crate::steps::voice_control) fn users_quiet_restart() -> Result<Vec<f32>> {
    let root = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?);
    let (session, start) = QUIET_RESTART;
    let form = forms_of(&root, session)?
        .into_iter()
        .find(|form| (form.start - start).abs() < 0.01)
        .with_context(|| format!("no form labelled at {start} s in {session}"))?;
    cut(&root, session, &form)
}

impl HandsFreeSession {
    /// Streams `samples` as a microphone captures them: a few milliseconds at a time.
    fn hear_in_real_time(&self, samples: &[f32]) -> Result<()> {
        let frame_time = Duration::from_secs_f32(REAL_TIME_FRAME_SAMPLES as f32 / SAMPLE_RATE_HZ);
        for frame in samples.chunks(REAL_TIME_FRAME_SAMPLES) {
            self.hear(frame)?;
            std::thread::sleep(frame_time);
        }
        Ok(())
    }

    /// Says `audio`, which starts a dictation, waits for the dictation to start and ends
    /// it with "stop dictation".
    fn dictate_by_voice(&self, audio: &[f32]) -> Result<()> {
        let room = silence(0.1);
        let seen = self.events().len();
        self.hear_in_real_time(audio)?;
        if !self.reacted(seen, &Expected::RecordingStarted, &room)? {
            bail!("no dictation started; events: {:?}", self.events());
        }
        let seen = self.events().len();
        self.hear(&command_audio(CommandVoice::Synthesized, "stop dictation")?)?;
        if !self.reacted(seen, &Expected::Pasted(SENTENCE), &room)? {
            bail!("nothing was pasted; events: {:?}", self.events());
        }
        Ok(())
    }

    fn last_pasted(&self) -> Result<String> {
        self.events()
            .into_iter()
            .rev()
            .find_map(|event| match event {
                Event::Pasted(text) => Some(text),
                _ => None,
            })
            .context("nothing was pasted")
    }
}

#[given("a dictation runs and someone has said a sentence in it")]
fn dictation_with_sentence(world: &mut E2eWorld) -> Result<()> {
    let session = session_of(world)?;
    let room = silence(0.1);
    let seen = session.events().len();
    session.hear(&command_audio(
        CommandVoice::Synthesized,
        "start dictation",
    )?)?;
    if !session.reacted(seen, &Expected::RecordingStarted, &room)? {
        bail!("no dictation started; events: {:?}", session.events());
    }
    session.hear_in_real_time(&spoken_by(ENGLISH_VOICE, BEFORE_RESTART)?)
}

#[given("a dictation started by voice runs")]
fn dictation_by_voice(world: &mut E2eWorld) -> Result<()> {
    let session = session_of(world)?;
    let seen = session.events().len();
    session.hear(&command_audio(
        CommandVoice::Synthesized,
        "start dictation",
    )?)?;
    if !session.reacted(seen, &Expected::RecordingStarted, &silence(0.1))? {
        bail!("no dictation started; events: {:?}", session.events());
    }
    Ok(())
}

#[when("someone says a sentence, takes a short breath and says \"stop dictation\"")]
fn says_sentence_breath_stop(world: &mut E2eWorld) -> Result<()> {
    let session = session_of(world)?;
    let mut audio = without_trailing_silence(from_first_sound(spoken_by(ENGLISH_VOICE, SENTENCE)?));
    audio.extend(silence(BREATH_SECS));
    audio.extend(from_first_sound(spoken_by(
        ENGLISH_VOICE,
        "stop dictation",
    )?));
    let seen = session.events().len();
    session.hear_in_real_time(&audio)?;
    // The Then step checks what was pasted; this only waits for the paste.
    session.reacted(seen, &Expected::Pasted(SENTENCE), &silence(0.1))?;
    Ok(())
}

#[when(expr = "someone says {string} and a sentence in one breath, and later ends the dictation")]
fn says_command_and_sentence(world: &mut E2eWorld, said: String) -> Result<()> {
    let audio = spoken_by(ENGLISH_VOICE, &format!("{said}, {SENTENCE}"))?;
    session_of(world)?.dictate_by_voice(&audio)
}

#[when(
    expr = "someone says {string}, pauses for about {float} s and says {word} sentence, and later ends the dictation"
)]
fn says_command_pause_sentence(
    world: &mut E2eWorld,
    said: String,
    pause_secs: f32,
    _which: String,
) -> Result<()> {
    let mut audio = without_trailing_silence(spoken_by(ENGLISH_VOICE, &said)?);
    audio.extend(silence(pause_secs));
    audio.extend(from_first_sound(spoken_by(ENGLISH_VOICE, SENTENCE)?));
    session_of(world)?.dictate_by_voice(&audio)
}

#[when(
    expr = "the user's recorded {string} is followed at once by {word} sentence, and the dictation later ends"
)]
fn recorded_command_and_sentence(world: &mut E2eWorld, said: String, _which: String) -> Result<()> {
    let mut audio = room_noise(SAY_PAUSE_SECS)?;
    audio.extend(recorded_form(&said)?);
    audio.extend(without_lead_pause(spoken_by(ENGLISH_VOICE, SENTENCE)?));
    session_of(world)?.dictate_by_voice(&audio)
}

/// Whether `pasted` holds a word of a command that starts a dictation.
fn holds_a_start_word(pasted: &str) -> bool {
    words(pasted)
        .iter()
        .any(|w| START_WORDS.contains(&w.as_str()))
}

/// Whether `pasted` holds every word of the sentence, its first words included.
fn holds_whole_sentence(pasted: &str) -> bool {
    let pasted_words = words(pasted);
    words(SENTENCE).iter().all(|w| pasted_words.contains(w))
}

#[then(expr = "the whole sentence, its first words included, is pasted without {string}")]
fn whole_sentence_pasted(world: &mut E2eWorld, command_word: String) -> Result<()> {
    let pasted = session_of(world)?.last_pasted()?;
    let has_command_word = words(&pasted).contains(&command_word) || holds_a_start_word(&pasted);
    if has_command_word || !holds_whole_sentence(&pasted) {
        bail!("pasted {pasted:?}, expected all of {SENTENCE:?} without {command_word:?}");
    }
    Ok(())
}

#[then("only the other sentence is pasted, its first words included")]
fn only_other_sentence(world: &mut E2eWorld) -> Result<()> {
    let pasted = session_of(world)?.last_pasted()?;
    let pasted_words = words(&pasted);
    let earlier_kept = words(BEFORE_RESTART)
        .iter()
        .any(|w| w.len() > 3 && pasted_words.contains(w));
    if earlier_kept || holds_a_start_word(&pasted) || !holds_whole_sentence(&pasted) {
        bail!("pasted {pasted:?}, expected only {SENTENCE:?}");
    }
    Ok(())
}
