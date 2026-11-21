//! The Claude dictation: a stand-in for Claude's own dictation records each key press the
//! pipeline asks of it, and can be told that Claude is not frontmost or that a press fails.

use cucumber::{given, then, when};
use voicepuncher::core::application::ClaudeDictation;
use voicepuncher::core::domain::RecordingMode;
use voicepuncher::test_doubles::prelude::*;

use super::doubles::{record, Event, Events};
use super::{listening_on_given, pipeline_log, run, VoiceRun, REACTION, SENTENCE, SETTLE};
use crate::IntegrationWorld;

/// Long enough to see that Return waits for Claude's text, short enough for a scenario.
pub const CLAUDE_TEXT_WAIT_IN_TESTS: Duration = Duration::from_millis(300);
/// The least pause between the stop press and Return the scenarios accept.
const LEAST_TEXT_WAIT: Duration = Duration::from_millis(250);
/// The start of the line the pipeline logs when Claude's dictation is not used.
const NOT_USED_LINE: &str = "claude dictation: not used:";

/// How the Claude dictation of a scenario behaves.
#[derive(Debug, Clone, Copy)]
pub struct ClaudeSetup {
    pub frontmost: bool,
    pub press_fails: bool,
    pub mode: RecordingMode,
}

impl ClaudeSetup {
    const IN_CLAUDE: Self =
        Self { frontmost: true, press_fails: false, mode: RecordingMode::Toggle };
}

pub struct StandInClaude {
    setup: ClaudeSetup,
    events: Events,
}

impl StandInClaude {
    pub fn new(setup: ClaudeSetup, events: &Events) -> Self {
        Self { setup, events: Arc::clone(events) }
    }

    fn press(&self, event: Event) -> Result<()> {
        if self.setup.press_fails {
            bail!("the key press cannot be posted");
        }
        record(&self.events, event);
        Ok(())
    }
}

impl ClaudeDictation for StandInClaude {
    fn is_frontmost(&self) -> bool {
        self.setup.frontmost
    }

    fn toggle(&mut self) -> Result<()> {
        self.press(Event::ClaudeShortcut)
    }

    fn cancel(&mut self) -> Result<()> {
        self.press(Event::ClaudeEscape)
    }
}

impl VoiceRun {
    fn claude_presses(&self) -> usize {
        self.since_mark()
            .iter()
            .filter(|e| **e == Event::ClaudeShortcut)
            .count()
    }

    fn wait_for_claude_presses(&self, wanted: usize) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        while self.claude_presses() < wanted {
            if Instant::now() >= deadline {
                bail!(
                    "Claude's shortcut pressed {} time(s), not {wanted}; events: {:?}",
                    self.claude_presses(),
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(SETTLE);
        ensure!(
            self.claude_presses() == wanted,
            "Claude's shortcut pressed {} times, not {wanted}; events: {:?}",
            self.claude_presses(),
            self.events()
        );
        Ok(())
    }

    fn not_used_lines(&self) -> Vec<String> {
        let Some(thread) = self.pipeline_thread else {
            return Vec::new();
        };
        let warnings = pipeline_log().warnings.lock().unwrap();
        warnings
            .iter()
            .filter(|(logged_by, line)| *logged_by == thread && line.starts_with(NOT_USED_LINE))
            .map(|(_, line)| line.clone())
            .collect()
    }
}

fn set_claude(world: &mut IntegrationWorld, setup: Option<ClaudeSetup>) {
    run(world).claude = setup;
}

#[given("the Claude dictation is turned on and the Claude desktop app is the frontmost app")]
fn in_claude(world: &mut IntegrationWorld) {
    set_claude(world, Some(ClaudeSetup::IN_CLAUDE));
}

#[given("the Claude dictation is turned on and another app is the frontmost app")]
fn in_another_app(world: &mut IntegrationWorld) {
    set_claude(
        world,
        Some(ClaudeSetup { frontmost: false, ..ClaudeSetup::IN_CLAUDE }),
    );
}

#[given("the Claude dictation is turned off and the Claude desktop app is the frontmost app")]
fn turned_off(world: &mut IntegrationWorld) {
    set_claude(world, None);
}

#[given("the Claude desktop app is the frontmost app and posting its dictation shortcut fails")]
fn press_fails(world: &mut IntegrationWorld) {
    set_claude(
        world,
        Some(ClaudeSetup { press_fails: true, ..ClaudeSetup::IN_CLAUDE }),
    );
}

#[given("the recording mode is Hold and the Claude desktop app is the frontmost app")]
fn hold_in_claude(world: &mut IntegrationWorld) {
    set_claude(
        world,
        Some(ClaudeSetup { mode: RecordingMode::Hold, ..ClaudeSetup::IN_CLAUDE }),
    );
}

#[given("a dictation started by voice in Claude runs")]
fn claude_dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    set_claude(world, Some(ClaudeSetup::IN_CLAUDE));
    listening_on_given(world)?;
    let run = run(world);
    run.start_when();
    run.say_alone(VoiceCommand::StartDictation.phrase())?;
    run.wait_for_claude_presses(1)?;
    run.speak(SENTENCE, Instant::now());
    Ok(())
}

/// Presses the hotkey, speaks once Claude's dictation started, then sends `end`.
fn hotkey_dictation_in_claude(world: &mut IntegrationWorld, end: PttSignal) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.press_hotkey()?;
    run.wait_for_claude_presses(1)?;
    run.speak(SENTENCE, Instant::now());
    run.send(PipelineInput::Hotkey(end))
}

#[when("the user presses the hotkey, speaks, and presses it again")]
fn toggles_in_claude(world: &mut IntegrationWorld) -> Result<()> {
    hotkey_dictation_in_claude(world, PttSignal::Pressed)
}

#[when("the user holds the hotkey, speaks, and releases it")]
fn holds_in_claude(world: &mut IntegrationWorld) -> Result<()> {
    hotkey_dictation_in_claude(world, PttSignal::Released)
}

#[then("Claude's dictation shortcut is pressed once and the app's own recorder does not start")]
fn pressed_once(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for_claude_presses(1)?;
    recorder_idle(world)
}

#[then(expr = "Claude's dictation shortcut is pressed twice")]
fn pressed_twice(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for_claude_presses(2)
}

#[then("Claude's dictation shortcut is pressed a second time")]
fn pressed_a_second_time(world: &mut IntegrationWorld) -> Result<()> {
    // the mark was set by the stop: the start's press came before it
    run(world).wait_for_claude_presses(1)
}

#[then("Claude's dictation shortcut is never pressed")]
fn never_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(
        |e| *e == Event::ClaudeShortcut,
        "Claude's shortcut was pressed",
    )
}

/// The app's own recorder did not start.
fn recorder_idle(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(
        |e| *e == Event::RecordingStarted,
        "the app's recorder started",
    )
}

#[then("nothing is pasted and no dictation is archived")]
fn nothing_pasted(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| matches!(e, Event::Pasted(_)), "a text was pasted")
}

#[then("Escape is pressed to drop Claude's dictation")]
fn escape_pressed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_since_mark(|e| *e == Event::ClaudeEscape)
}

#[then("Escape is pressed, then Claude's dictation shortcut")]
fn escape_then_shortcut(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for_claude_presses(1)?;
    let events = run.since_mark();
    let escape = events.iter().position(|e| *e == Event::ClaudeEscape);
    let shortcut = events.iter().position(|e| *e == Event::ClaudeShortcut);
    ensure!(
        escape.is_some() && escape < shortcut,
        "not Escape, then the shortcut: {events:?}"
    );
    Ok(())
}

#[then("Claude's dictation shortcut is pressed, then Return after a pause")]
fn shortcut_then_return(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == Event::ClaudeShortcut)?;
    let pressed_at = Instant::now();
    run.wait_since_mark(|e| *e == Event::ReturnPressed)?;
    ensure!(
        pressed_at.elapsed() >= LEAST_TEXT_WAIT,
        "Return came {:?} after the stop press",
        pressed_at.elapsed()
    );
    Ok(())
}

#[then("the voice start is timed in the log as for the app's own dictation")]
fn voice_start_timed(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let thread = run.pipeline_thread.context("the pipeline is not running")?;
    let deadline = Instant::now() + REACTION;
    loop {
        let starts = pipeline_log().voice_starts.lock().unwrap();
        if starts.iter().any(|(logged_by, _)| *logged_by == thread) {
            return Ok(());
        }
        drop(starts);
        ensure!(
            Instant::now() < deadline,
            "no voice start line; events: {:?}",
            run.events()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[then("no Escape is pressed")]
fn no_escape(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| *e == Event::ClaudeEscape, "Escape was pressed")
}

#[then("the log names why the Claude dictation was not used")]
fn not_used_logged(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let deadline = Instant::now() + REACTION;
    while run.not_used_lines().is_empty() {
        ensure!(
            Instant::now() < deadline,
            "no {NOT_USED_LINE:?} line; events: {:?}",
            run.events()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
