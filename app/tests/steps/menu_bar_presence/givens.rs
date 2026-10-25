//! Given steps of the menu bar.

use super::{
    commands_away, config_with, given, recording_mode, run, set, IconState, IntegrationWorld,
    Result,
};

#[given("the app was just launched and its runtime is still starting")]
#[given("the status item shows the waiting icon while the runtime starts")]
#[given("the runtime is starting")]
pub(super) fn runtime_starting(world: &mut IntegrationWorld) {
    run(world).start_held();
}

#[given("the runtime is ready and no dictation runs")]
pub(super) fn runtime_ready(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_ready()
}

#[given("a button has keyboard focus while the runtime is ready")]
pub(super) fn button_focused(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.text_field_focused, false);
    run.start_ready()
}

#[given("the speech recognition service is not running while a recording runs")]
pub(super) fn speech_service_down(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.transcription_fails, true);
    run.start_ready()?;
    run.start_recording()
}

#[given("voice control is turned on and a recording runs")]
pub(super) fn voice_control_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.config.voice_control.enabled = true;
    run.start_ready()?;
    run.start_recording()
}

#[given("voice control is turned on and no dictation runs yet")]
pub(super) fn voice_control_on(world: &mut IntegrationWorld) {
    run(world).config.voice_control.enabled = true;
}

#[given("voice control is turned on, the runtime is ready and no dictation runs")]
pub(super) fn voice_control_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.config.voice_control.enabled = true;
    run.start_ready()
}

#[given("the commands-unavailable sign shows")]
pub(super) fn commands_unavailable_shows(world: &mut IntegrationWorld) -> Result<()> {
    voice_control_ready(world)?;
    commands_away(world)?;
    run(world).icon_is(IconState::CommandsUnavailable)
}

#[given(expr = "the runtime is ready with the hotkey {string} in the recording mode {string}")]
pub(super) fn ready_with_hotkey(
    world: &mut IntegrationWorld,
    key: String,
    mode: String,
) -> Result<()> {
    let run = run(world);
    run.config = config_with(key.parse()?, recording_mode(&mode)?);
    run.start_ready()
}

#[given(expr = "a recording runs with the hotkey {string} in the recording mode {string}")]
pub(super) fn recording_with_hotkey(
    world: &mut IntegrationWorld,
    key: String,
    mode: String,
) -> Result<()> {
    ready_with_hotkey(world, key, mode)?;
    run(world).start_recording()
}

#[given("the runtime failed because the speech recognition service does not answer")]
pub(super) fn runtime_failed(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    set(&run.parts.services_answer, false);
    run.start();
    run.icon_is(IconState::Failed)
}

#[given("the menu was opened and closed while the runtime was ready")]
pub(super) fn menu_opened_while_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_ready()?;
    run.open_menu();
    Ok(())
}
