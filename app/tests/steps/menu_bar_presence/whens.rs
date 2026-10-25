//! When steps of the menu bar.

use super::{
    config_with, run, set, when, Arc, CommandPlacement, Context, HotKey, IconState, Instant,
    IntegrationWorld, MacosMenuActions, MenuActions, Mutex, PipelineInput, QuitEvent,
    RecordingMode, Result, SettingsOpener, SpottingEvent, VoiceCommand,
};

#[when("the menu bar is looked at")]
pub(super) fn menu_bar_looked_at(_world: &mut IntegrationWorld) {}

#[when("the runtime becomes ready")]
pub(super) fn runtime_becomes_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.starting.is_some() {
        run.finish_start(true)
    } else {
        run.start();
        run.started_ok()
    }
}

#[when("the runtime restarts with a new configuration and becomes ready again")]
pub(super) fn runtime_restarts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.drawn.before = run.mark();
    let config = config_with(HotKey::F19, RecordingMode::Toggle);
    run.runtime()?.reload(config)
}

#[when("the runtime fails because a local service does not answer")]
pub(super) fn runtime_fails(world: &mut IntegrationWorld) -> Result<()> {
    run(world).finish_start(false)
}

#[when("the user presses the hotkey to start a recording")]
pub(super) fn user_starts_recording(world: &mut IntegrationWorld) {
    run(world).press();
}

#[when("the user dictates a sentence, ends the dictation and then clicks into a text field")]
pub(super) fn dictation_through_every_stage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.drawn.before = run.mark();
    run.start_recording()?;
    run.press();
    run.icon_is(IconState::WaitingForFocus)?;
    set(&run.parts.text_field_focused, true);
    Ok(())
}

#[when("the user stops the dictation with the hotkey")]
pub(super) fn user_stops_dictation(world: &mut IntegrationWorld) {
    run(world).press();
}

#[when("the user clears the text by voice")]
pub(super) fn user_clears_by_voice(world: &mut IntegrationWorld) -> Result<()> {
    let now = Instant::now();
    run(world).send(PipelineInput::Spotting(SpottingEvent::CommandsSaid {
        commands: vec![VoiceCommand::ClearAllText],
        start: now,
        end: now,
        placement: CommandPlacement::Alone,
        timing: None,
    }))
}

#[when("the command spotting service has been away long enough for the commands-unavailable sign")]
pub(super) fn commands_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingStillAway)
}

#[when("the user opens the menu")]
pub(super) fn user_opens_menu(world: &mut IntegrationWorld) {
    run(world).open_menu();
}

#[when("the user opens the menu again during speech recognition")]
pub(super) fn menu_during_recognition(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.transcription_held, true);
    run.start_recording()?;
    run.press();
    run.icon_is(IconState::Processing)?;
    run.open_menu();
    Ok(())
}

#[when("the user chooses Quit in the menu")]
pub(super) fn user_quits(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let runtime = run.runtime.take().context("the runtime was not started")?;
    let quit_log = Arc::clone(&run.chosen.quit_log);
    let end_process = move |code| quit_log.lock().unwrap().push(QuitEvent::ProcessEnded(code));
    let scratch = std::env::temp_dir();
    let actions = MacosMenuActions::ending_with(
        runtime,
        Box::new(SettingsOpens(Arc::default())),
        scratch.join("logs"),
        Box::new(end_process),
    );
    actions.quit();
    run.actions = Some(actions);
    Ok(())
}

/// Counts how often the menu opened the settings window.
pub(super) struct SettingsOpens(pub(crate) Arc<Mutex<usize>>);

impl SettingsOpener for SettingsOpens {
    fn open(&self) {
        *self.0.lock().unwrap() += 1;
    }
}

#[when("the user chooses Settings in the menu")]
pub(super) fn user_chooses_settings(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let runtime = run.runtime.take().context("the runtime was not started")?;
    let opens = Arc::clone(&run.chosen.settings_opens);
    let actions = MacosMenuActions::ending_with(
        runtime,
        Box::new(SettingsOpens(opens)),
        std::env::temp_dir().join("logs"),
        Box::new(|_| {}),
    );
    actions.open_settings();
    run.actions = Some(actions);
    Ok(())
}
