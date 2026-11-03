//! When steps: what the user edits.

use super::{
    app_runtime, settings, text, when, IntegrationWorld, Result, Value, DEBUG_LEVEL,
    REMOTE_SERVICE, SECOND_START_SOUND, SILENT_LOCAL_SERVICE, TOO_SHORT_PRESS_MS,
};

#[when("the user picks the second hotkey and the second start sound and saves")]
pub(super) fn pick_hotkey_and_sound(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("hotkey", "key", text("F19"))?;
    run.edit("sounds", "start_sound", text(SECOND_START_SOUND))?;
    run.save()
}

#[when("the user picks the second hotkey and saves")]
pub(super) fn pick_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("hotkey", "key", text("F19"))?;
    run.save()
}

#[when("the user saves a speech recognition address on this Mac where no service answers")]
pub(super) fn save_silent_address(world: &mut IntegrationWorld) -> Result<()> {
    app_runtime::run(world).services_stop_answering();
    let run = settings(world);
    run.edit("transcription", "base_url", text(SILENT_LOCAL_SERVICE))?;
    run.save()
}

#[when("the user enters a minimum press of 5 milliseconds and saves")]
pub(super) fn too_short_press(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("hotkey", "min_press_ms", Value::Integer(TOO_SHORT_PRESS_MS))?;
    run.save()
}

#[when("the user enters a speech recognition address on another machine and saves")]
pub(super) fn remote_address(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("transcription", "base_url", text(REMOTE_SERVICE))?;
    run.save()
}

#[when("the user changes the logging level and saves")]
pub(super) fn change_logging_level(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("logging", "level", text(DEBUG_LEVEL))?;
    run.save()
}

#[when("the user chooses Revert")]
pub(super) fn revert(world: &mut IntegrationWorld) -> Result<()> {
    settings(world).session()?.revert();
    Ok(())
}

#[when("the user closes the settings")]
pub(super) fn close(_world: &mut IntegrationWorld) {
    // closing asks only when the session has unsaved edits; the Then steps read that
}

#[when("the options the configuration file accepts are compared with the settings controls")]
pub(super) fn compare_options(_world: &mut IntegrationWorld) {
    // the Then step compares the configuration's keys with the field table
}
