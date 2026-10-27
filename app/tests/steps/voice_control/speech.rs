//! When steps: what the user says and presses.

use super::{
    bail, given, run, when, CommandPlacement, Context, Instant, IntegrationWorld, PipelineInput,
    PttSignal, Result, SpottingEvent, VoiceCommand, VoiceRun, COMMAND_AFTER, COMMAND_LENGTH,
    LATE_CUT, MORE_WORDS, PHRASE_LEAD, PHRASE_LENGTH, SEGMENT_TAIL, SENTENCE, SETTLE, STOP_PHRASE,
};
use voicepuncher::test_doubles::StartedOnDemand;

#[when("the user presses the hotkey once")]
pub(super) fn press_hotkey_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.press_hotkey()
}

#[when(expr = "the user says a sentence and then {string} without a pause, then pauses")]
pub(super) fn says_sentence_then_command(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    let (start, command_at) = begin_sentence(run);
    run.speak(&said, command_at);
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(start)))?;
    let commands = chain(&said).context("no command")?;
    let end = command_at + COMMAND_LENGTH;
    run.say_commands(
        &commands,
        command_at + LATE_CUT,
        end,
        CommandPlacement::Tail,
    )?;
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechEnded(end)))
}

#[when(expr = "the user says a sentence, {string} and more words, without any pause")]
pub(super) fn says_doubled_in_speech(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    let (start, command_at) = begin_sentence(run);
    let end = command_at + COMMAND_LENGTH;
    run.speak(&said, command_at);
    run.speak(MORE_WORDS, end);
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(start)))?;
    let commands = doubled(&said).context("no doubled command")?;
    run.say_commands(
        &commands,
        command_at + LATE_CUT,
        end,
        CommandPlacement::Anywhere,
    )
}

#[when(expr = "the user says a sentence and {string} without a pause")]
pub(super) fn says_sentence_and_doubled(world: &mut IntegrationWorld, said: String) -> Result<()> {
    let run = run(world);
    let (start, command_at) = begin_sentence(run);
    run.send(PipelineInput::Spotting(SpottingEvent::SpeechStarted(start)))?;
    let commands = doubled(&said).context("no doubled command")?;
    run.say_commands(
        &commands,
        command_at,
        command_at + COMMAND_LENGTH,
        CommandPlacement::Anywhere,
    )
}

#[when("the user presses Escape")]
pub(super) fn press_escape(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.send(PipelineInput::Hotkey(PttSignal::Cancel))
}

#[when("the user records a dictation with the hotkey")]
pub(crate) fn records_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.dictate_with_hotkey()?;
    run.press_hotkey()?;
    run.pasted().map(drop)
}

#[when(expr = "the user says {string}")]
#[when(expr = "the user says {string} before the text is pasted")]
pub(super) fn says(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    match chain(&phrase) {
        Some(commands) if commands.len() > 1 => {
            let start = Instant::now();
            run.say_commands(
                &commands,
                start,
                start + COMMAND_LENGTH,
                CommandPlacement::Anywhere,
            )
        }
        Some(commands) if commands.len() == 1 => run.say_alone(commands[0].phrase()),
        _ => run.say_alone(&phrase),
    }
}

/// The command a short form names.
pub(super) fn short_form(word: &str) -> Option<VoiceCommand> {
    match word {
        "start" => Some(VoiceCommand::StartDictation),
        "stop" => Some(VoiceCommand::StopDictation),
        "cancel" => Some(VoiceCommand::CancelDictation),
        "restart" => Some(VoiceCommand::RestartDictation),
        "enter" => Some(VoiceCommand::PressEnter),
        "clear" => Some(VoiceCommand::ClearAllText),
        "paste" => Some(VoiceCommand::PasteText),
        _ => None,
    }
}

/// The commands of a chain of short forms ("stop enter start").
/// Starts a new stretch of speech with the sentence; when it started and when a command said
/// after it starts.
fn begin_sentence(run: &mut VoiceRun) -> (Instant, Instant) {
    run.start_when();
    let start = Instant::now();
    run.speak(SENTENCE, start);
    (start, start + COMMAND_AFTER)
}

/// The one command `said` is, said once or doubled.
pub(super) fn one_command(said: &str) -> Result<VoiceCommand> {
    match chain(said).context("no command")?.as_slice() {
        [command] => Ok(*command),
        [first, second] if first == second => Ok(*first),
        _ => bail!("{said:?} is not one command"),
    }
}

pub(super) fn chain(said: &str) -> Option<Vec<VoiceCommand>> {
    said.split_whitespace().map(short_form).collect()
}

/// The command a doubled short form ("stop stop") names.
pub(super) fn doubled(said: &str) -> Option<Vec<VoiceCommand>> {
    match chain(said)?.as_slice() {
        [first, second] if first == second => Some(vec![*first]),
        _ => None,
    }
}

#[when(expr = "the user says {string} without pausing")]
pub(super) fn says_in_sentence(world: &mut IntegrationWorld, sentence: String) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.say_in_sentence(&sentence, STOP_PHRASE)
}

#[when("the command spotting service stops answering")]
pub(super) fn service_stops(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.send(PipelineInput::SpottingLost)
}

#[given("the command spotting service went away during it")]
pub(super) fn service_went_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.away_since = Some(Instant::now());
    run.send(PipelineInput::SpottingLost)?;
    std::thread::sleep(SETTLE);
    Ok(())
}

#[given("the command spotting service went away and is back")]
pub(crate) fn service_went_away_and_is_back(world: &mut IntegrationWorld) -> Result<()> {
    service_went_away(world)?;
    let run = run(world);
    run.send(PipelineInput::SpottingBack)?;
    std::thread::sleep(SETTLE);
    Ok(())
}

#[when(expr = "the service is back and reports {string}, said while it was away")]
pub(super) fn back_with_command_said_away(
    world: &mut IntegrationWorld,
    phrase: String,
) -> Result<()> {
    let run = run(world);
    let away_since = run.away_since.context("the service did not go away")?;
    run.start_when();
    run.send(PipelineInput::SpottingBack)?;
    let segment_start = away_since + PHRASE_LEAD;
    let phrase_start = segment_start + PHRASE_LEAD;
    let phrase_end = phrase_start + PHRASE_LENGTH;
    run.speak(&phrase, phrase_start);
    run.send_segment(
        &phrase,
        segment_start..phrase_end + SEGMENT_TAIL,
        phrase_start..phrase_end,
    )
}
