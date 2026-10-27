//! How soon a voice start gives its start cue, end to end: the command streams in real time,
//! as from a microphone, and the cue is timed from the moment the sample that ends the
//! command's words was fed to the moment the stand-in sound player got the start (or restart)
//! sound.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher_e2e_test::machine_load;
use voicepuncher_e2e_test::stalled_copies::{self, Watch};
use voicepuncher_e2e_test::voice_commands::{
    from_first_sound, silence, spoken_by, without_trailing_silence,
};

use super::speech_after_start::{BEFORE_RESTART, SENTENCE};
use super::{env, session_of, Cues, Expected, HandsFreeSession, ENGLISH_VOICE, SAMPLE_RATE_HZ};
use crate::E2eWorld;

/// 20 ms per message: a real microphone delivers audio about this often.
const REAL_TIME_FRAME_SAMPLES: usize = 320;
/// Longest a start cue is waited for: one this late fails the scenario anyway.
const CUE_WAIT: Duration = Duration::from_secs(5);
/// The voices "cancel dictation" is said in, one per try, before the scenario gives up.
/// WHY: decoding is deterministic, so the same audio misheard once ("Capsule dictation")
/// is misheard the same way again; another voice is a new chance.
const CANCEL_VOICES: [&str; 2] = [ENGLISH_VOICE, "Samantha"];
/// How long a dictation runs before the "restart" said on its own.
/// Said during the dictation before a restart, one after another.
const DICTATED: [&str; 3] = [
    BEFORE_RESTART,
    "Please bring the slides to the review on Monday",
    "The train leaves at seven and arrives before noon",
];

/// The bound of a voice start's cue, and of a doubled form, heard as a command without
/// waiting for the pause after it or the next word.
const CUE_BOUND: Duration = Duration::from_millis(700);
const DOUBLED_CUE_BOUND: Duration = Duration::from_millis(500);

/// One voice start the scenario timed.
#[derive(Debug, Clone)]
pub(super) struct MeasuredStart {
    /// What started the dictation and what followed it, for the report.
    label: String,
    /// When the sample that ends the command's words was fed.
    words_ended: Instant,
    /// Where the command's words end in the stream the command spotting service heard, in
    /// seconds: its log places each confirmation there.
    words_end_in_stream: f32,
    /// When the last sample of the speech after the command was fed, if speech followed.
    speech_ended: Option<Instant>,
    pub(super) cue: Option<Instant>,
    /// The confirmations the command spotting service cut short while the cue was awaited.
    cut_short: Vec<String>,
}

impl MeasuredStart {
    fn took(&self) -> Option<Duration> {
        self.cue
            .map(|cue| cue.saturating_duration_since(self.words_ended))
    }

    fn within(&self, bound: Duration) -> bool {
        self.took().is_some_and(|took| took <= bound)
    }

    /// The bound of this start's cue: shorter for a doubled form, read from the words its
    /// label quotes.
    fn bound(&self) -> Duration {
        let said = self.label.split('"').nth(1).unwrap_or_default();
        let words: Vec<&str> = said.split_whitespace().collect();
        if matches!(words.as_slice(), [first, second] if first == second) {
            DOUBLED_CUE_BOUND
        } else {
            CUE_BOUND
        }
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
                "{}: no start cue (words end at {:.2} s of the stream); {}",
                self.label,
                self.words_end_in_stream,
                stalled_copies::report(&self.cut_short)
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
            let due = began + frame_time * u32::try_from(n).unwrap();
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
    pub(super) fn time_start(
        &self,
        label: String,
        audio: &[f32],
        words_end: usize,
        room: &[f32],
    ) -> Result<MeasuredStart> {
        self.time_cue(label, (audio, words_end), room, &self.cues)
    }

    /// Says `said`, audio whose command's words end at the sample it names, and times the
    /// first of `cues` given after it.
    pub(super) fn time_cue(
        &self,
        label: String,
        (audio, words_end): (&[f32], usize),
        room: &[f32],
        cues: &Cues,
    ) -> Result<MeasuredStart> {
        let cues_before = cues.lock().unwrap().len();
        let heard_before = self.microphone.captured.lock().unwrap().len();
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let watch = env("VOICEPUNCHER_COMMAND_SPOTTING_URL").map(|url| Watch::start(&url, &home));
        let (words_ended, fed) = self.hear_timed(audio, words_end)?;
        let cue = self.next_cue(cues, cues_before, room, fed)?;
        let measured = MeasuredStart {
            label,
            words_ended,
            words_end_in_stream: (heard_before + words_end) as f32 / SAMPLE_RATE_HZ,
            speech_ended: (audio.len() > words_end).then_some(fed),
            cue,
            cut_short: watch.map(|watch| watch.cut_short()).unwrap_or_default(),
        };
        // The numbers of design.md § Measurements come from these lines.
        println!("start cue | {}", measured.report());
        self.starts.lock().unwrap().push(measured.clone());
        Ok(measured)
    }

    /// The first start cue after the first `seen` cues, while the microphone keeps capturing
    /// the room in real time after the frame fed at `last_fed`; none when it does not come
    /// within `CUE_WAIT`.
    fn next_cue(
        &self,
        cues: &Cues,
        seen: usize,
        room: &[f32],
        last_fed: Instant,
    ) -> Result<Option<Instant>> {
        let deadline = Instant::now() + CUE_WAIT;
        let frame_time = Duration::from_secs_f32(REAL_TIME_FRAME_SAMPLES as f32 / SAMPLE_RATE_HZ);
        for (n, frame) in room.chunks(REAL_TIME_FRAME_SAMPLES).cycle().enumerate() {
            let cue = cues.lock().unwrap().get(seen).copied();
            if let Some(cue) = cue {
                return Ok(Some(cue));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            // WHY: a sleep of one frame after each frame runs slower than real time, so the
            // pause after a command would reach the service later than it was said.
            let due = last_fed + frame_time * (u32::try_from(n).unwrap() + 1);
            std::thread::sleep(due.saturating_duration_since(Instant::now()));
            self.hear(frame)?;
        }
        Ok(None)
    }

    /// Ends a dictation the scenario started, without transcribing it: after a pause, so
    /// "cancel dictation" is an utterance of its own, and once more in another voice when
    /// it was not heard.
    pub(super) fn cancel_dictation(&self, room: &[f32]) -> Result<()> {
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

    /// The last voice start, which must be the one `said` began.
    pub(super) fn last_start_of(&self, said: &str) -> Result<MeasuredStart> {
        let start = self.last_start()?;
        if !start.label.contains(said) {
            bail!("the last voice start was {}, not \"{said}\"", start.label);
        }
        Ok(start)
    }
}

/// `said` in a synthesized voice, cut after its last sound.
pub(super) fn synthesized_command(said: &str) -> Result<Vec<f32>> {
    Ok(without_trailing_silence(spoken_by(ENGLISH_VOICE, said)?))
}

pub(super) fn sentence_at_once() -> Result<Vec<f32>> {
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

#[given(expr = "someone has dictated for about {float} s")]
fn dictated_for(world: &mut E2eWorld, secs: f32) -> Result<()> {
    let session = session_of(world)?;
    let mut audio = Vec::new();
    for sentence in DICTATED.iter().cycle() {
        if audio.len() as f32 >= secs * SAMPLE_RATE_HZ {
            break;
        }
        audio.extend(spoken_by(ENGLISH_VOICE, sentence)?);
    }
    session.hear_timed(&audio, audio.len()).map(|_| ())
}

#[when(expr = "someone says {string} and a/another sentence without a pause")]
fn says_command_and_sentence_at_once(world: &mut E2eWorld, said: String) -> Result<()> {
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

#[then(
    expr = "the start cue comes within {float} s of the end of {string}, while the sentence is still being said"
)]
fn cue_while_speaking(world: &mut E2eWorld, bound_secs: f32, said: String) -> Result<()> {
    let start = session_of(world)?.last_start_of(&said)?;
    within_while_speaking(&start, bound_secs)
}

/// `start`'s cue came within `bound_secs` of its words, before the speech after them ended.
pub(super) fn within_while_speaking(start: &MeasuredStart, bound_secs: f32) -> Result<()> {
    check_within(start, bound_secs)?;
    match (start.cue, start.speech_ended) {
        (Some(cue), Some(speech_ended)) if cue < speech_ended => Ok(()),
        _ => bail!(
            "the cue came after the sentence was said; {}",
            start.report()
        ),
    }
}

#[then(expr = "the start cue comes within {float} s of the end of {string}")]
fn cue_within(world: &mut E2eWorld, bound_secs: f32, said: String) -> Result<()> {
    check_within(&session_of(world)?.last_start_of(&said)?, bound_secs)
}

#[then(expr = "the start cue of the new dictation comes within {float} s of the end of {string}")]
fn restart_cue_within(world: &mut E2eWorld, bound_secs: f32, said: String) -> Result<()> {
    check_within(&session_of(world)?.last_start_of(&said)?, bound_secs)
}

#[then(
    expr = "at least {int} of {int} of them give the start cue within its bound of the end of the command's words"
)]
fn most_recorded_within(world: &mut E2eWorld, wanted: usize, of: usize) -> Result<()> {
    let starts = session_of(world)?.starts.lock().unwrap().clone();
    let in_time = starts
        .iter()
        .filter(|start| start.within(start.bound()))
        .count();
    if in_time * of < wanted * starts.len() {
        let reports: Vec<String> = starts.iter().map(MeasuredStart::report).collect();
        bail!(
            "{in_time} of {} recorded starts gave the cue within their bound:\n{}\n{}",
            starts.len(),
            reports.join("\n"),
            machine_load::report()
        );
    }
    Ok(())
}
