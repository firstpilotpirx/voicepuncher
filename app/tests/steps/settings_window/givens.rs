//! Given steps: the configuration file and the open window.

use super::{
    app_runtime, files, given, settings, text, when, Arc, ConfigApplier, FakeMicrophones,
    IntegrationWorld, NoApp, Result, RuntimeApplier, SessionParts, SettingsSession,
    COMMAND_LINE_HOTKEY, MICROPHONES,
};

#[given("the configuration file sets the hotkey to the right Command key")]
pub(super) fn file_sets_right_command(world: &mut IntegrationWorld) {
    settings(world).write(files::RIGHT_COMMAND);
}

#[given("the configuration file does not set the minimum press length")]
pub(super) fn file_without_min_press(world: &mut IntegrationWorld) {
    settings(world).write(files::WITHOUT_MIN_PRESS);
}

#[given("the configuration file holds a valid minimum press length")]
pub(super) fn file_with_min_press(world: &mut IntegrationWorld) {
    settings(world).write(files::VALID_MIN_PRESS);
}

#[given("a configuration file with comments above and beside its options")]
pub(super) fn file_with_comments(world: &mut IntegrationWorld) {
    settings(world).write(files::COMMENTED);
}

#[given("a configuration file that still holds the retired text refinement settings")]
pub(super) fn file_with_retired_settings(world: &mut IntegrationWorld) {
    settings(world).write(files::WITH_RETIRED);
}

#[given("no configuration file exists")]
pub(super) fn no_file(world: &mut IntegrationWorld) {
    settings(world).original = None;
}

#[given("the Mac has two input devices")]
pub(super) fn two_microphones(world: &mut IntegrationWorld) {
    settings(world).microphones = MICROPHONES.map(str::to_owned).to_vec();
}

#[given(expr = "the app was started with the hotkey F13 on the command line")]
pub(super) fn hotkey_on_command_line(world: &mut IntegrationWorld) {
    settings(world)
        .command_line
        .push(|config| config.hotkey.key = COMMAND_LINE_HOTKEY);
}

#[given("the app runs from its configuration file with the first hotkey and the first start sound")]
pub(super) fn app_runs_first(world: &mut IntegrationWorld) -> Result<()> {
    app_runs_from(world, files::FIRST_SETTINGS)
}

pub(super) fn app_runs_from(world: &mut IntegrationWorld, file: &str) -> Result<()> {
    let run = settings(world);
    run.write(file);
    let config = voicepuncher::config::load(&run.path())?;
    let app = app_runtime::run(world);
    let pastes = app.paste_counter();
    app.start_from(config)?;
    settings(world).pastes = Some(Arc::new(pastes));
    Ok(())
}

#[given("the user opens the settings")]
#[when("the user opens the settings")]
pub(super) fn open_settings(world: &mut IntegrationWorld) -> Result<()> {
    let applier: Box<dyn ConfigApplier> = match world.app.runtime.as_ref() {
        Some(run) => Box::new(RuntimeApplier::new(run.reloader()?)),
        None => Box::new(NoApp),
    };
    let run = settings(world);
    let command_line = run.command_line.clone();
    let parts = SessionParts {
        overlay: Box::new(move |config| command_line.iter().for_each(|set| set(config))),
        applier,
        devices: Box::new(FakeMicrophones(run.microphones.clone())),
    };
    run.session = Some(SettingsSession::open(
        &run.path(),
        run.folder.path(),
        parts,
    )?);
    Ok(())
}

#[given("the user changed the hotkey without saving")]
pub(super) fn hotkey_changed(world: &mut IntegrationWorld) -> Result<()> {
    settings(world).edit("hotkey", "key", text("F19"))
}
