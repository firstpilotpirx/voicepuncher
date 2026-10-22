//! How soon a voice start gives its start cue, end to end: the command streams in real time,
//! as from a microphone, and the cue is timed from the moment the sample that ends the
//! command's words was fed to the moment the stand-in sound player got the start (or restart)
//! sound.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher_e2e_test::machine_load;
use voicepuncher_e2e_test::voice_commands::{
    from_first_sound, silence, spoken_by, without_trailing_silence,
};

use super::speech_after_start::{cut, forms_of, BEFORE_RESTART, SENTENCE};
use super::{
    env, room_noise, session_of, until_last_sound, Event, Expected, HandsFreeSession,
    ENGLISH_VOICE, SAMPLE_RATE_HZ,
};
use crate::E2eWorld;

/// 20 ms per message: a real microphone delivers audio about this often.
const REAL_TIME_FRAME_SAMPLES: usize = 320;
/// Longest a start cue is waited for: one this late fails the scenario anyway.
const CUE_WAIT: Duration = Duration::from_secs(5);
/// The voices "cancel dictation" is said in, one per try, before the scenario gives up.
/// WHY: decoding is deterministic, so the same audio misheard once ("Capsule dictation")
/// is misheard the same way again; another voice is a new chance.
const CANCEL_VOICES: [&str; 2] = [ENGLISH_VOICE, "Samantha"];
/// Room noise before each of the user's recorded starts, as in a real stream.
const ROOM_BEFORE_SECS: f32 = 1.0;
/// How long a dictation runs before the "restart" said on its own.
const DICTATED_SECS: f32 = 20.0;
/// Said during the dictation before a restart, one after another.
const DICTATED: [&str; 3] = [
    BEFORE_RESTART,
    "Please bring the slides to the review on Monday",
    "The train leaves at seven and arrives before noon",
];
/// The user's labelled recording session holding their starts from standby.
const RECORDED_STARTS_SESSION: &str = "session-2";
/// The ways of saying a start from standby that are only the command's words.
const START_FORMS: [&str; 3] = ["start", "start start", "start please"];

/// One voice start the scenario timed.
#[derive(Debug, Clone)]
pub(super) struct MeasuredStart {
    /// What started the dictation and what followed it, for the report.
    label: String,
    /// The events of the session before the command was said.
    events_before: usize,
    /// When the sample that ends the command's words was fed.
    words_ended: Instant,
    /// Where the command's words end in the stream the command spotting service heard, in
    /// seconds: its log places each confirmation there.
    words_end_in_stream: f32,
    /// When the last sample of the speech after the command was fed, if speech followed.
    speech_ended: Option<Instant>,
    cue: Option<Instant>,
}

impl MeasuredStart {
    fn took(&self) -> Option<Duration> {
        self.cue
            .map(|cue| cue.saturating_duration_since(self.words_ended))
    }

    fn within(&self, bound: Duration) -> bool {
        self.took().is_some_and(|took| took <= bound)
    }

    fn report(&self) -> String {
        match self.took() {
            Some(took) => format!(
                "{}: {} ms (words end at {:.2} s of the stream)",
                self.label,
                took.as_millis(),
                self.words_end_in_stream
            ),
            None => format!(
                "{}: no start cue (words end at {:.2} s of the stream)",
                self.label, self.words_end_in_stream
            ),
        }
    }
}

impl HandsFreeSession {
    /// Streams `samples` in real time and returns when the sample at `words_end` (the first
    /// one past the command's words) and the last sample were fed.
    fn hear_timed(&self, samples: &[f32], words_end: usize) -> Result<(Instant, Instant)> {
        let frame_time = Duration::from_secs_f32(REAL_TIME_FRAME_SAMPLES as f32 / SAMPLE_RATE_HZ);
        let began = Instant::now();
        let mut words_ended = None;
        for (n, frame) in samples.chunks(REAL_TIME_FRAME_SAMPLES).enumerate() {
            let due = began + frame_time * n as u32;
            std::thread::sleep(due.saturating_duration_since(Instant::now()));
            self.hear(frame)?;
            if words_ended.is_none() && (n + 1) * REAL_TIME_FRAME_SAMPLES >= words_end {
                words_ended = Some(Instant::now());
            }
        }
        let fed = Instant::now();
        Ok((words_ended.unwrap_or(fed), fed))
    }

    /// Says `audio`, whose command's words end at sample `words_end`, then lets the room be
    /// heard until a dictation starts, and times the start cue.
    fn time_start(
        &self,
        label: String,
        audio: &[f32],
        words_end: usize,
        room: &[f32],
    ) -> Result<MeasuredStart> {
        let events_before = self.events().len();
        let cues_before = self.cues.lock().unwrap().len();
        let heard_before = self.microphone.captured.lock().unwrap().len();
        let (words_ended, fed) = self.hear_timed(audio, words_end)?;
        let cue = self.next_cue(cues_before, room, fed)?;
        let measured = MeasuredStart {
            label,
            events_before,
            words_ended,
            words_end_in_stream: (heard_before + words_end) as f32 / SAMPLE_RATE_HZ,
            speech_ended: (audio.len() > words_end).then_some(fed),
            cue,
        };
        // The numbers of design.md § Measurements come from these lines.
        println!("start cue | {}", measured.report());
        self.starts.lock().unwrap().push(measured.clone());
        Ok(measured)
    }

    /// The first start cue after the first `seen` cues, while the microphone keeps capturing
    /// the room in real time after the frame fed at `last_fed`; none when it does not come
    /// within `CUE_WAIT`.
    fn next_cue(&self, seen: usize, room: &[f32], last_fed: Instant) -> Result<Option<Instant>> {
        let deadline = Instant::now() + CUE_WAIT;
        let frame_time = Duration::from_secs_f32(REAL_TIME_FRAME_SAMPLES as f32 / SAMPLE_RATE_HZ);
        for (n, frame) in room.chunks(REAL_TIME_FRAME_SAMPLES).cycle().enumerate() {
            if let Some(cue) = self.cues.lock().unwrap().get(seen).copied() {
                return Ok(Some(cue));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            // WHY: a sleep of one frame after each frame runs slower than real time, so the
            // pause after a command would reach the service later than it was said.
            let due = last_fed + frame_time * (n as u32 + 1);
            std::thread::sleep(due.saturating_duration_since(Instant::now()));
            self.hear(frame)?;
        }
        Ok(None)
    }

    /// Ends a dictation the scenario started, without transcribing it: after a pause, so
    /// "cancel dictation" is an utterance of its own, and once more in another voice when
    /// it was not heard.
    fn cancel_dictation(&self, room: &[f32]) -> Result<()> {
        for voice in CANCEL_VOICES {
            let seen = self.events().len();
            self.hear_timed(room, room.len())?;
            self.hear(&spoken_by(voice, "cancel dictation")?)?;
            if self.reacted(seen, &Expected::Cancelled, &silence(0.1))? {
                return Ok(());
            }
        }
        bail!(
            "the dictation was not cancelled; events: {:?}",
            self.events()
        )
    }

    fn last_start(&self) -> Result<MeasuredStart> {
        self.starts
            .lock()
            .unwrap()
            .last()
            .cloned()
            .context("no voice start was timed")
    }
}

/// `said` in a synthesized voice, cut after its last sound.
fn synthesized_command(said: &str) -> Result<Vec<f32>> {
    Ok(without_trailing_silence(spoken_by(ENGLISH_VOICE, said)?))
}

fn sentence_at_once() -> Result<Vec<f32>> {
    Ok(from_first_sound(spoken_by(ENGLISH_VOICE, SENTENCE)?))
}

/// A cue over its bound fails naming what ran beside it: the spec bounds the cue on an
/// idle machine, so the load at the failure tells a slow confirmation from a loaded machine.
fn check_within(start: &MeasuredStart, bound_secs: f32) -> Result<()> {
    if !start.within(Duration::from_secs_f32(bound_secs)) {
        bail!(
            "expected the start cue within {bound_secs} s of the end of the command's words; {}\n{}",
            start.report(),
            machine_load::report()
        );
    }
    Ok(())
}

#[given(expr = "someone has dictated for about {int} s")]
fn dictated_for(world: &mut E2eWorld, _secs: u32) -> Result<()> {
    let session = session_of(world)?;
    let mut audio = Vec::new();
    for sentence in DICTATED.iter().cycle() {
        if audio.len() as f32 >= DICTATED_SECS * SAMPLE_RATE_HZ {
            break;
        }
        audio.extend(spoken_by(ENGLISH_VOICE, sentence)?);
    }
    session.hear_timed(&audio, audio.len()).map(|_| ())
}

#[when(expr = "someone says {string} and {word} sentence without a pause")]
fn says_command_and_sentence_at_once(
    world: &mut E2eWorld,
    said: String,
    _which: String,
) -> Result<()> {
    let session = session_of(world)?;
    let mut audio = synthesized_command(&said)?;
    let words_end = audio.len();
    audio.extend(sentence_at_once()?);
    let label = format!("\"{said}\" followed by speech");
    session.time_start(label, &audio, words_end, &silence(0.1))?;
    Ok(())
}

#[when(expr = "someone says {string} and stays silent")]
fn says_command_alone(world: &mut E2eWorld, said: String) -> Result<()> {
    let session = session_of(world)?;
    let audio = synthesized_command(&said)?;
    let label = format!("\"{said}\" alone");
    session.time_start(label, &audio, audio.len(), &silence(0.1))?;
    Ok(())
}

#[when(
    "each of the user's recorded starts is played, on its own and followed at once by a sentence"
)]
fn recorded_starts(world: &mut E2eWorld) -> Result<()> {
    let session = session_of(world)?;
    let root = PathBuf::from(env("VOICEPUNCHER_VOICE_SAMPLES")?);
    let forms: Vec<_> = forms_of(&root, RECORDED_STARTS_SESSION)?
        .into_iter()
        .filter(|form| {
            form.state == "standby"
                && form.commands == ["start dictation"]
                && START_FORMS.contains(&form.said.as_str())
        })
        .collect();
    if forms.is_empty() {
        bail!("no recorded starts in {RECORDED_STARTS_SESSION}");
    }
    let room = room_noise(0.8)?;
    for form in &forms {
        for followed in [false, true] {
            let mut audio = room_noise(ROOM_BEFORE_SECS)?;
            audio.extend(until_last_sound(cut(&root, RECORDED_STARTS_SESSION, form)?));
            let words_end = audio.len();
            if followed {
                audio.extend(sentence_at_once()?);
            }
            let after = if followed {
                "followed by speech"
            } else {
                "alone"
            };
            let label = format!("recorded \"{}\" at {} s {after}", form.said, form.start);
            let start = session.time_start(label, &audio, words_end, &room)?;
            if start.cue.is_some() {
                session.cancel_dictation(&room)?;
            }
        }
    }
    Ok(())
}

#[then(
    expr = "the start cue comes within {float} s of the end of {string}, while the sentence is still being said"
)]
fn cue_while_speaking(world: &mut E2eWorld, bound_secs: f32, _said: String) -> Result<()> {
    let start = session_of(world)?.last_start()?;
    check_within(&start, bound_secs)?;
    match (start.cue, start.speech_ended) {
        (Some(cue), Some(speech_ended)) if cue < speech_ended => Ok(()),
        _ => bail!(
            "the start cue came after the sentence was said; {}",
            start.report()
        ),
    }
}

#[then(expr = "the start cue comes within {float} s of the end of {string}")]
fn cue_within(world: &mut E2eWorld, bound_secs: f32, _said: String) -> Result<()> {
    check_within(&session_of(world)?.last_start()?, bound_secs)
}

#[then(expr = "the start cue of the new dictation comes within {float} s of the end of {string}")]
fn restart_cue_within(world: &mut E2eWorld, bound_secs: f32, _said: String) -> Result<()> {
    check_within(&session_of(world)?.last_start()?, bound_secs)
}

#[then(
    expr = "the Return key is pressed and the start cue comes within {float} s of the end of {string}"
)]
fn return_and_cue_within(world: &mut E2eWorld, bound_secs: f32, _said: String) -> Result<()> {
    let session = session_of(world)?;
    let start = session.last_start()?;
    if !session.events()[start.events_before..].contains(&Event::ReturnPressed) {
        bail!(
            "the Return key was not pressed; events: {:?}",
            session.events()
        );
    }
    check_within(&start, bound_secs)
}

#[then(
    expr = "at least {int} of {int} of them give the start cue within {float} s of the end of the command's words"
)]
fn most_recorded_within(
    world: &mut E2eWorld,
    wanted: usize,
    of: usize,
    bound_secs: f32,
) -> Result<()> {
    let starts = session_of(world)?.starts.lock().unwrap().clone();
    let bound = Duration::from_secs_f32(bound_secs);
    let in_time = starts.iter().filter(|start| start.within(bound)).count();
    if in_time * of < wanted * starts.len() {
        let reports: Vec<String> = starts.iter().map(MeasuredStart::report).collect();
        bail!(
            "{in_time} of {} recorded starts gave the cue within {bound_secs} s:\n{}\n{}",
            starts.len(),
            reports.join("\n"),
            machine_load::report()
        );
    }
    Ok(())
}
