//! Given and When steps of the runtime's start, reloads and stop.

use super::{
    bail, config_with, given, run, when, Context, Event, IntegrationWorld, Result, FIRST_HOTKEY,
    REMOTE_SERVICE, SECOND_HOTKEY, SETTLE, THIRD_HOTKEY,
};

#[given("a valid configuration with voice control off")]
#[given("a valid configuration")]
pub(super) fn valid_configuration(world: &mut IntegrationWorld) {
    let run = run(world);
    run.config = config_with(FIRST_HOTKEY);
    run.config.voice_control.enabled = false;
}

#[given("the speech recognition service answers")]
pub(super) fn services_answer(world: &mut IntegrationWorld) {
    *run(world).services_answer.lock().unwrap() = true;
}

#[given("the speech recognition service does not answer")]
pub(super) fn service_silent(world: &mut IntegrationWorld) {
    *run(world).services_answer.lock().unwrap() = false;
}

#[given("the runtime is Ready with the first hotkey")]
#[given("the runtime is Ready")]
pub(super) fn runtime_ready(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_ready()
}

#[given("the runtime is Ready with voice control on")]
pub(super) fn runtime_ready_with_voice_control(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.config.voice_control.enabled = true;
    run.start_ready()?;
    run.wait_for(|e| matches!(e, Event::ListeningOn(_)))?;
    Ok(())
}

#[given("a recording is in progress")]
pub(super) fn recording_in_progress(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if run.runtime.is_none() {
        run.start_ready()?;
    }
    run.start_recording()?;
    Ok(())
}

#[given("a dictation waits for a text input to gain focus")]
pub(super) fn dictation_waits_for_focus(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    *run.text_field_focused.lock().unwrap() = false;
    run.start_recording()?;
    run.keyboard.press(run.config.hotkey.key);
    std::thread::sleep(SETTLE);
    if run.count(|e| matches!(e, Event::Pasted(_))) > 0 {
        bail!("the dictation was pasted without focus: {:?}", run.events());
    }
    Ok(())
}

#[when("the runtime starts")]
pub(super) fn runtime_starts(world: &mut IntegrationWorld) {
    run(world).start();
}

#[when("the runtime reloads a valid configuration with a second hotkey")]
#[when("the runtime reloads a valid configuration")]
pub(super) fn reload_second_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.reload(config_with(SECOND_HOTKEY))
}

#[when("the runtime reloads two valid configurations one right after the other")]
pub(super) fn reload_twice(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.reload(config_with(SECOND_HOTKEY))?;
    run.reload(config_with(THIRD_HOTKEY))
}

#[when("the runtime reloads a configuration that does not validate")]
pub(super) fn reload_invalid(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let mut config = config_with(SECOND_HOTKEY);
    REMOTE_SERVICE.clone_into(&mut config.transcription.base_url);
    let log = crate::steps::voice_control::pipeline_log();
    let mark = log.warnings.lock().unwrap().len();
    let here = std::thread::current().id();
    if run.reload(config).is_ok() {
        bail!("the runtime accepted a configuration that does not validate");
    }
    run.observed.rejection_log = log.warnings.lock().unwrap()[mark..]
        .iter()
        .filter(|(thread, _)| *thread == here)
        .map(|(_, line)| line.clone())
        .collect();
    Ok(())
}

#[when("the runtime stops")]
pub(super) fn runtime_stops(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.runtime
        .as_mut()
        .context("the runtime was not started")?
        .stop();
    Ok(())
}
