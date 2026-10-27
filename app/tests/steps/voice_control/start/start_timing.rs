//! The timing line a voice start logs: the scripted service reports how each command's
//! report came about, as the adapter does by the clock of the microphone capture, and the
//! scenarios read the lines the pipeline's own thread logged.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::core::domain::{ConfirmationTiming, ReportTiming};

use super::doubles::VOICE_START_LINE;
use super::speech_after_start::{sleep_until, CONFIRMATION_TIME};
use super::{
    one_command, pipeline_log, run, Event, VoiceRun, COMMAND_LENGTH, PHRASE_LEAD, REACTION,
    SEGMENT_TAIL, SETTLE,
};
use crate::IntegrationWorld;

/// How much microphone audio never reached the service: the stream lags the clock by it.
const LOST_AUDIO: Duration = Duration::from_secs(3);
/// How long after the end of its segment the service reports a command said alone.
const REPORTED_AFTER_SEGMENT: Duration = Duration::from_millis(300);
/// How far the logged total may be from the time the scenario measured.
const TOTAL_TOLERANCE: Duration = Duration::from_millis(100);
/// How soon after the start cue the overlay's recording sign must show.
const OVERLAY_AFTER_CUE: Duration = Duration::from_millis(50);
/// The stages of the timing line, after its total.
const STAGES: [&str; 5] = ["hear", "confirm", "deliver", "cue", "overlay"];

impl VoiceRun {
    /// The timing lines the scenario's pipeline logged.
    fn voice_start_lines(&self) -> Vec<String> {
        let Some(thread) = self.pipeline_thread else {
            return Vec::new();
        };
        pipeline_log()
            .voice_starts
            .lock()
            .unwrap()
            .iter()
            .filter(|(logged_by, _)| *logged_by == thread)
            .map(|(_, line)| line.clone())
            .collect()
    }

    /// The first timing line, and when it was seen.
    fn first_voice_start_line(&self) -> Result<(String, Instant)> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(line) = self.voice_start_lines().into_iter().next() {
                return Ok((line, Instant::now()));
            }
            if Instant::now() >= deadline {
                bail!(
                    "no voice start timing line within {REACTION:?}; events: {:?}",
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// When the first event since the mark that `wanted` matches was seen.
    fn seen_since_mark(&self, wanted: impl Fn(&Event) -> bool) -> Result<Instant> {
        let deadline = Instant::now() + REACTION;
        while !self.since_mark().iter().any(&wanted) {
            if Instant::now() >= deadline {
                bail!("not seen within {REACTION:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(Instant::now())
    }

    /// Says `said` on its own and stays silent; the service reports it once its segment
    /// ended, `confirmed` or not, dated by a stream that lags the clock by the audio lost.
    fn say_alone_and_stay_silent(&mut self, said: &str, confirmed: bool) -> Result<()> {
        let command = one_command(said)?;
        let segment_start = Instant::now();
        let start = segment_start + PHRASE_LEAD;
        let end = start + COMMAND_LENGTH;
        let segment_end = end + SEGMENT_TAIL;
        self.command_end = Some(end);
        sleep_until(segment_end + REPORTED_AFTER_SEGMENT);
        let in_stream = |moment: Instant| moment.checked_sub(self.audio_lost).unwrap();
        let report = ReportTiming {
            words_ended: end,
            confirmation: confirmed
                .then_some(ConfirmationTiming { asked_at: segment_end, took: CONFIRMATION_TIME }),
            arrived_at: Instant::now(),
        };
        let events = [
            SpottingEvent::SpeechStarted(in_stream(segment_start)),
            SpottingEvent::PhraseSpotted {
                phrase: command.phrase().to_owned(),
                start: in_stream(start),
                end: in_stream(end),
                timing: Some(report),
            },
            SpottingEvent::SpeechEnded(in_stream(segment_end)),
        ];
        for event in events {
            self.send(PipelineInput::Spotting(event))?;
        }
        Ok(())
    }
}

/// The milliseconds of each field of a timing line, `None` for an unknown one.
fn fields(line: &str) -> Result<Vec<(String, Option<u64>)>> {
    let fields = line
        .strip_prefix(VOICE_START_LINE)
        .with_context(|| format!("not a timing line: {line:?}"))?;
    fields
        .split_whitespace()
        .map(|field| field_ms(field, line))
        .collect()
}

/// One `name=12ms` field of `line`, `None` for `name=-`.
fn field_ms(field: &str, line: &str) -> Result<(String, Option<u64>)> {
    let (name, value) = field
        .split_once('=')
        .with_context(|| format!("no value in {field:?} of {line:?}"))?;
    let ms = match value {
        "-" => None,
        _ => Some(
            value
                .strip_suffix("ms")
                .and_then(|ms| ms.parse().ok())
                .with_context(|| format!("no milliseconds in {field:?} of {line:?}"))?,
        ),
    };
    Ok((name.to_owned(), ms))
}

fn total_ms(line: &str) -> Result<u64> {
    match fields(line)?.first() {
        Some((name, Some(ms))) if name == "total" => Ok(*ms),
        _ => bail!("no total first in {line:?}"),
    }
}

#[when(expr = "the user says {string} and {word} sentence without a pause")]
fn says_command_and_sentence_on(
    world: &mut IntegrationWorld,
    said: String,
    which: String,
) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_command_then_sentence(&said, &which)
}

#[given("some seconds of microphone audio never reached the command spotting service earlier")]
fn audio_lost(world: &mut IntegrationWorld) {
    run(world).audio_lost = LOST_AUDIO;
}

#[when(expr = "the user says {string} and stays silent")]
fn says_alone_and_stays_silent(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_alone_and_stay_silent(&said, true)
}

#[when(
    expr = "the user says {string}, reported without the Whisper confirmation, and stays silent"
)]
fn says_alone_unconfirmed(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_alone_and_stay_silent(&said, false)
}

#[then("the timing line of that start shows that it was not confirmed")]
fn timing_line_unconfirmed(world: &mut IntegrationWorld) -> Result<()> {
    let (line, _) = run(world).first_voice_start_line()?;
    if !line
        .split_whitespace()
        .any(|field| field == "confirm=unconfirmed")
    {
        bail!("the timing line does not say the start was not confirmed: {line:?}");
    }
    Ok(())
}

#[then(
    "the log shows one timing line for that start, with its total and the time of each stage, the cue sound and the overlay included"
)]
fn one_timing_line(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let (line, _) = run.first_voice_start_line()?;
    std::thread::sleep(SETTLE);
    let lines = run.voice_start_lines();
    if lines.len() != 1 {
        bail!("expected one timing line, got {lines:?}");
    }
    let fields = fields(&line)?;
    let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    let expected: Vec<&str> = std::iter::once("total").chain(STAGES).collect();
    if names != expected || fields.iter().any(|(_, ms)| ms.is_none()) {
        bail!("expected the total and every stage in milliseconds: {line:?}");
    }
    Ok(())
}

#[then(
    "the logged total matches the time from the end of the command to the start cue within 0.1 s"
)]
fn total_matches(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let (line, seen_at) = run.first_voice_start_line()?;
    run.wait_since_mark(|e| *e == Event::Overlay("recording"))?;
    let end = run.command_end.context("no command was said")?;
    let measured = seen_at.saturating_duration_since(end);
    let logged = Duration::from_millis(total_ms(&line)?);
    if logged.abs_diff(measured) > TOTAL_TOLERANCE {
        bail!("logged {line:?}, but the start cue came {measured:?} after the command's end");
    }
    Ok(())
}

#[then("the log shows no voice start timing line")]
fn no_timing_line(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::RecordingStarted)?;
    std::thread::sleep(SETTLE);
    let lines = run.voice_start_lines();
    if !lines.is_empty() {
        bail!("a hotkey start logged a voice start timing: {lines:?}");
    }
    Ok(())
}

#[then("the overlay's recording sign shows within 0.05 s of the start cue")]
fn overlay_with_cue(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let cue = run.seen_since_mark(|e| *e == Event::Sound("start"))?;
    let shown = run.seen_since_mark(|e| *e == Event::Overlay("recording"))?;
    let after = shown.saturating_duration_since(cue);
    let events = run.since_mark();
    let at = |wanted: &Event| events.iter().position(|e| e == wanted);
    let waited_for_microphone = matches!(
        (at(&Event::RecordingStarted), at(&Event::Overlay("recording"))),
        (Some(started), Some(shown)) if started < shown
    );
    if after > OVERLAY_AFTER_CUE || waited_for_microphone {
        bail!("the recording sign showed {after:?} after the start cue; events: {events:?}");
    }
    Ok(())
}
