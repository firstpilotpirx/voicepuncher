//! Speech said right after a voice command that starts a dictation. The command spotting
//! service reports such a command only once the next words have begun, as it does when it
//! waits for the pause after a command or searches a running utterance, so the scenarios
//! report it late.

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{then, when};

use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::{CommandPlacement, SpottingEvent};

use super::{
    chain, doubled, run, Event, VoiceRun, COMMAND_LENGTH, OTHER_SENTENCE, PHRASE_LEAD, REACTION,
    SEGMENT_TAIL, SENTENCE,
};
use crate::IntegrationWorld;

/// When words follow a command without a pause, how soon after its end they begin.
const NO_PAUSE: Duration = Duration::from_millis(50);
/// A short pause after a command: just longer than the silence that ends an utterance.
const SHORT_PAUSE: Duration = Duration::from_millis(500);
/// How long after the next words began the service reports the command.
const REPORTED_AFTER: Duration = Duration::from_millis(300);

/// The sentence a step names: "a sentence" or "another sentence".
fn sentence_named(which: &str) -> Result<&'static str> {
    match which {
        "a" => Ok(SENTENCE),
        "another" => Ok(OTHER_SENTENCE),
        other => bail!("no sentence named {other:?}"),
    }
}

fn sleep_until(moment: Instant) {
    std::thread::sleep(moment.saturating_duration_since(Instant::now()));
}

impl VoiceRun {
    /// Ends the dictation the scenario's command started, as the user later does.
    fn end_started_dictation(&mut self) -> Result<()> {
        self.wait_since_mark(|e| *e == Event::RecordingStarted)?;
        self.press_hotkey()
    }

    /// The first paste after event `from` that holds `sentence`.
    fn pasted_after(&self, from: usize, sentence: &str) -> Result<String> {
        let deadline = Instant::now() + REACTION;
        loop {
            let pasted = self.events()[from..].iter().find_map(|e| match e {
                Event::Pasted(text) if text.contains(sentence) => Some(text.clone()),
                _ => None,
            });
            if let Some(text) = pasted {
                return Ok(text);
            }
            if Instant::now() >= deadline {
                bail!("no paste holds {sentence:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[when(
    expr = "the user says {string} and {word} sentence without a pause, and later ends the dictation"
)]
fn says_command_and_sentence(
    world: &mut IntegrationWorld,
    said: String,
    which: String,
) -> Result<()> {
    let run = run(world);
    run.start_when();
    let (commands, placement) = match (doubled(&said), chain(&said).context("no command")?) {
        (Some(once), _) => (once, CommandPlacement::Anywhere),
        (None, chained) if chained.len() > 1 => (chained, CommandPlacement::Anywhere),
        (None, alone) => (alone, CommandPlacement::Head),
    };
    let start = Instant::now();
    let end = start + COMMAND_LENGTH;
    run.speak(&said, start);
    run.speak(sentence_named(&which)?, end + NO_PAUSE);
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(start)))?;
    sleep_until(end + NO_PAUSE + REPORTED_AFTER);
    run.say_commands(&commands, start, end, placement)?;
    run.end_started_dictation()
}

#[when(
    expr = "the user says {string}, pauses for about 0.5 s and says {word} sentence, and later ends the dictation"
)]
fn says_command_pause_sentence(
    world: &mut IntegrationWorld,
    said: String,
    which: String,
) -> Result<()> {
    let run = run(world);
    run.start_when();
    let command = match chain(&said).context("no command")?.as_slice() {
        [command] => *command,
        _ => bail!("{said:?} is not one command"),
    };
    let segment_start = Instant::now();
    let start = segment_start + PHRASE_LEAD;
    let end = start + COMMAND_LENGTH;
    run.speak(&said, start);
    run.speak(sentence_named(&which)?, end + SHORT_PAUSE);
    sleep_until(end + SHORT_PAUSE + REPORTED_AFTER);
    run.send_segment(
        command.phrase(),
        segment_start,
        start,
        end,
        end + SEGMENT_TAIL,
    )?;
    run.end_started_dictation()
}

#[then("the whole sentence, its first words included, is pasted into the focused app")]
fn whole_sentence_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.pasted_after(run.mark, SENTENCE)?;
    Ok(())
}

#[then("the other sentence, its first words included, is pasted after the Return key is pressed")]
fn other_sentence_pasted_after_return(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::ReturnPressed)?;
    let since = run.since_mark();
    let returned_at = since
        .iter()
        .position(|e| *e == Event::ReturnPressed)
        .context("no Return")?;
    run.pasted_after(run.mark + returned_at, OTHER_SENTENCE)?;
    Ok(())
}
