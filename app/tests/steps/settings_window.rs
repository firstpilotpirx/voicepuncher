//! Drives a settings session over a real configuration file in a temporary folder; when a
//! scenario runs the app, saved settings reach the app-runtime stand-ins through the real
//! runtime, as the menu bar applies them.

mod files;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};
use cucumber::{given, then, when};
use toml::Value;

use voicepuncher::adapters::primary::menubar::macos::applier::RuntimeApplier;
use voicepuncher::adapters::primary::settings::macos::field::{Choices, Field};
use voicepuncher::adapters::primary::settings::macos::session::{
    status_text, ApplyStatus, ConfigApplier, Refusal, SessionParts, SettingsSession, StatusListener,
};
use voicepuncher::adapters::primary::settings::macos::table::find;
use voicepuncher::adapters::primary::settings::macos::values::display;
use voicepuncher::config::Config;
use voicepuncher::core::application::InputDevices;
use voicepuncher::core::domain::HotKey;

use crate::steps::app_runtime;
use crate::IntegrationWorld;

const FIRST_HOTKEY: HotKey = HotKey::RightOption;
const SECOND_HOTKEY: HotKey = HotKey::F19;
const COMMAND_LINE_HOTKEY: HotKey = HotKey::F13;
const SECOND_START_SOUND: &str = "Hero";
const SILENT_LOCAL_SERVICE: &str = "http://127.0.0.1:9/v1";
const REMOTE_SERVICE: &str = "http://10.0.0.7:8081/v1";
const TOO_SHORT_PRESS_MS: i64 = 5;
const DEBUG_LEVEL: &str = "debug";
const MICROPHONES: [&str; 2] = ["USB Mic", "AirPods"];
/// Longest the runtime may take to tell what became of a Save.
const REACTION: Duration = Duration::from_secs(2);

/// What a settings scenario shares between its steps.
pub struct SettingsRun {
    folder: tempfile::TempDir,
    original: Option<String>,
    microphones: Vec<String>,
    command_line: Vec<fn(&mut Config)>,
    session: Option<SettingsSession>,
    statuses: Arc<Mutex<Vec<ApplyStatus>>>,
    /// Counts the app's pastes, when the scenario runs the app.
    pastes: Option<Arc<dyn Fn() -> usize + Send + Sync>>,
    /// The pastes the app had made when the settings were said to be applied.
    pastes_when_applied: Arc<Mutex<Option<usize>>>,
    refusal: Option<Refusal>,
}

impl std::fmt::Debug for SettingsRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsRun")
            .field("statuses", &self.statuses)
            .field("refusal", &self.refusal)
            .finish()
    }
}

impl Default for SettingsRun {
    fn default() -> Self {
        Self {
            folder: tempfile::tempdir().expect("a temporary folder"),
            original: None,
            microphones: Vec::new(),
            command_line: Vec::new(),
            session: None,
            statuses: Arc::default(),
            pastes: None,
            pastes_when_applied: Arc::default(),
            refusal: None,
        }
    }
}

impl SettingsRun {
    fn path(&self) -> PathBuf {
        self.folder.path().join("config.toml")
    }

    fn write(&mut self, text: &str) {
        std::fs::write(self.path(), text).expect("the configuration file is written");
        self.original = Some(text.to_owned());
    }

    fn file(&self) -> Result<String> {
        std::fs::read_to_string(self.path()).context("the configuration file is not there")
    }

    fn session(&mut self) -> Result<&mut SettingsSession> {
        self.session
            .as_mut()
            .context("the settings were not opened")
    }

    fn shown(&mut self, section: &str, key: &str) -> Result<String> {
        let field = field(section, key);
        Ok(display(field, &self.session()?.value(field)))
    }

    fn edit(&mut self, section: &str, key: &str, value: Value) -> Result<()> {
        self.session()?.edit(field(section, key), value);
        Ok(())
    }

    fn save(&mut self) -> Result<()> {
        let statuses = Arc::clone(&self.statuses);
        let pastes = self.pastes.clone();
        let when_applied = Arc::clone(&self.pastes_when_applied);
        let listener: StatusListener = Box::new(move |s| {
            if let (ApplyStatus::Applied, Some(pastes)) = (&s, &pastes) {
                *when_applied.lock().unwrap() = Some(pastes());
            }
            statuses.lock().unwrap().push(s)
        });
        self.refusal = self.session()?.save(listener).err();
        Ok(())
    }

    fn wait_for_status(&self, wanted: impl Fn(&ApplyStatus) -> bool) -> Result<ApplyStatus> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(status) = self.statuses.lock().unwrap().iter().find(|s| wanted(s)) {
                return Ok(status.clone());
            }
            ensure!(Instant::now() < deadline, "statuses: {:?}", self.statuses);
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn marked(&self) -> Result<String> {
        let refusal = self.refusal.as_ref().context("the Save was not refused")?;
        Ok(refusal.field.context("no field is marked")?.path())
    }
}

fn field(section: &str, key: &str) -> &'static Field {
    find(section, key).unwrap_or_else(|| panic!("no field for {section}.{key}"))
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn settings(world: &mut IntegrationWorld) -> &mut SettingsRun {
    world.settings.get_or_insert_with(SettingsRun::default)
}

/// Saved settings with no app running: applied at once.
struct NoApp;

impl ConfigApplier for NoApp {
    fn apply(&self, _config: Config, mut listener: StatusListener) {
        listener(ApplyStatus::Applied);
    }
}

struct FakeMicrophones(Vec<String>);

impl InputDevices for FakeMicrophones {
    fn names(&self) -> Vec<String> {
        self.0.clone()
    }
}

#[given("the configuration file sets the hotkey to the right Command key")]
fn file_sets_right_command(world: &mut IntegrationWorld) {
    settings(world).write(files::RIGHT_COMMAND);
}

#[given("the configuration file does not set the minimum press length")]
fn file_without_min_press(world: &mut IntegrationWorld) {
    settings(world).write(files::WITHOUT_MIN_PRESS);
}

#[given("the configuration file holds a valid minimum press length")]
fn file_with_min_press(world: &mut IntegrationWorld) {
    settings(world).write(files::VALID_MIN_PRESS);
}

#[given("the configuration file turns refinement on")]
fn file_refinement_on(world: &mut IntegrationWorld) {
    settings(world).write(files::REFINEMENT_ON);
}

#[given("a configuration file with comments above and beside its options")]
fn file_with_comments(world: &mut IntegrationWorld) {
    settings(world).write(files::COMMENTED);
}

#[given("a configuration file with refinement examples")]
fn file_with_examples(world: &mut IntegrationWorld) {
    settings(world).write(files::WITH_EXAMPLES);
}

#[given("no configuration file exists")]
fn no_file(world: &mut IntegrationWorld) {
    settings(world).original = None;
}

#[given("the Mac has two input devices")]
fn two_microphones(world: &mut IntegrationWorld) {
    settings(world).microphones = MICROPHONES.map(str::to_owned).to_vec();
}

#[given(expr = "the app was started with the hotkey F13 on the command line")]
fn hotkey_on_command_line(world: &mut IntegrationWorld) {
    settings(world)
        .command_line
        .push(|config| config.hotkey.key = COMMAND_LINE_HOTKEY);
}

#[given("the app was started with refinement turned off on the command line")]
fn no_llm_on_command_line(world: &mut IntegrationWorld) {
    settings(world)
        .command_line
        .push(|config| config.refinement.enabled = false);
}

#[given("the app runs from its configuration file with the first hotkey and the first start sound")]
fn app_runs_first(world: &mut IntegrationWorld) -> Result<()> {
    app_runs_from(world, files::FIRST_SETTINGS)
}

#[given("the app runs from its configuration file with refinement off")]
fn app_runs_unrefined(world: &mut IntegrationWorld) -> Result<()> {
    app_runs_from(world, files::FIRST_SETTINGS)
}

fn app_runs_from(world: &mut IntegrationWorld, file: &str) -> Result<()> {
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
fn open_settings(world: &mut IntegrationWorld) -> Result<()> {
    let applier: Box<dyn ConfigApplier> = match world.runtime.as_ref() {
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
fn hotkey_changed(world: &mut IntegrationWorld) -> Result<()> {
    settings(world).edit("hotkey", "key", text("F19"))
}

#[when("the user picks the second hotkey and the second start sound and saves")]
fn pick_hotkey_and_sound(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("hotkey", "key", text("F19"))?;
    run.edit("sounds", "start_sound", text(SECOND_START_SOUND))?;
    run.save()
}

#[when("the user picks the second hotkey and saves")]
fn pick_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("hotkey", "key", text("F19"))?;
    run.save()
}

#[when("the user switches refinement on and saves")]
fn switch_refinement_on(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("refinement", "enabled", Value::Boolean(true))?;
    run.save()
}

#[when("the user saves a speech recognition address on this Mac where no service answers")]
fn save_silent_address(world: &mut IntegrationWorld) -> Result<()> {
    app_runtime::run(world).services_stop_answering();
    let run = settings(world);
    run.edit("transcription", "base_url", text(SILENT_LOCAL_SERVICE))?;
    run.save()
}

#[when("the user enters a minimum press of 5 milliseconds and saves")]
fn too_short_press(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("hotkey", "min_press_ms", Value::Integer(TOO_SHORT_PRESS_MS))?;
    run.save()
}

#[when("the user enters a speech recognition address on another machine and saves")]
fn remote_address(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("transcription", "base_url", text(REMOTE_SERVICE))?;
    run.save()
}

#[when("the user changes the logging level and saves")]
fn change_logging_level(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    run.edit("logging", "level", text(DEBUG_LEVEL))?;
    run.save()
}

#[when("the user chooses Revert")]
fn revert(world: &mut IntegrationWorld) -> Result<()> {
    settings(world).session()?.revert();
    Ok(())
}

#[when("the user closes the settings")]
fn close(_world: &mut IntegrationWorld) {
    // closing asks only when the session has unsaved edits; the Then steps read that
}

#[when("the options the configuration file accepts are compared with the settings controls")]
fn compare_options(_world: &mut IntegrationWorld) {
    // the Then step compares the configuration's keys with the field table
}

#[then("the hotkey control shows Right Command")]
fn shows_right_command(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("hotkey", "key")?;
    ensure!(shown == HotKey::RightCommand.human_name(), "shows {shown}");
    Ok(())
}

#[then("the minimum press control shows the default length")]
fn shows_default_press(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("hotkey", "min_press_ms")?;
    let default = Config::default().hotkey.min_press_ms.to_string();
    ensure!(shown == default, "shows {shown}, the default is {default}");
    Ok(())
}

#[then("every option has exactly one control")]
fn every_option_has_a_control(_world: &mut IntegrationWorld) -> Result<()> {
    files::every_key_has_one_field()
}

#[then("the microphone choice offers the system default and both devices by name")]
fn microphone_choice(world: &mut IntegrationWorld) -> Result<()> {
    let offered = settings(world).session()?.options(Choices::Microphones);
    let titles: Vec<&str> = offered.iter().map(|c| c.title.as_str()).collect();
    ensure!(
        titles == ["System default", MICROPHONES[0], MICROPHONES[1]],
        "{titles:?}"
    );
    Ok(())
}

#[then("the configuration file holds the second hotkey and the second start sound")]
fn file_holds_second(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(saved.hotkey.key == SECOND_HOTKEY, "{:?}", saved.hotkey.key);
    ensure!(saved.sounds.start_sound == SECOND_START_SOUND);
    Ok(())
}

#[then("the settings say the settings are applied")]
fn says_applied(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let status = run.wait_for_status(|s| *s == ApplyStatus::Applied)?;
    ensure!(
        status_text(&status) == "Applied",
        "{}",
        status_text(&status)
    );
    Ok(())
}

#[then("once the dictation is pasted the settings say the settings are applied")]
fn says_applied_after_the_paste(world: &mut IntegrationWorld) -> Result<()> {
    app_runtime::restarted_after_the_dictation(app_runtime::run(world))?;
    says_applied(world)?;
    let pastes = *settings(world).pastes_when_applied.lock().unwrap();
    ensure!(
        pastes == Some(1),
        "the settings were said to be applied after {pastes:?} pastes"
    );
    Ok(())
}

#[then("the next dictation starts with the second hotkey and plays the second start sound")]
fn next_dictation_second(world: &mut IntegrationWorld) -> Result<()> {
    let app = app_runtime::run(world);
    let (sound, _) = app.newest_setup().context("no pipeline was assembled")?;
    ensure!(sound == SECOND_START_SOUND, "plays {sound}");
    ensure!(
        app.hotkey_records(SECOND_HOTKEY),
        "the second hotkey started no recording"
    );
    app.finish_dictation(SECOND_HOTKEY)
}

#[then("the next dictation is refined")]
fn next_dictation_refined(world: &mut IntegrationWorld) -> Result<()> {
    settings(world).wait_for_status(|s| *s == ApplyStatus::Applied)?;
    let app = app_runtime::run(world);
    let (_, refines) = app.newest_setup().context("no pipeline was assembled")?;
    ensure!(refines, "the newest pipeline does not refine");
    ensure!(app.hotkey_records(FIRST_HOTKEY), "no dictation started");
    app.finish_dictation(FIRST_HOTKEY)
}

#[then("the app was not relaunched")]
fn not_relaunched(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(
        app_runtime::run(world).same_process(),
        "the process changed"
    );
    Ok(())
}

#[then("the configuration file holds the new address")]
fn file_holds_new_address(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(saved.transcription.base_url == SILENT_LOCAL_SERVICE);
    Ok(())
}

#[then("the settings say the settings were not applied and name the speech recognition service")]
fn says_not_applied(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let status = run.wait_for_status(|s| matches!(s, ApplyStatus::Rejected(_)))?;
    let said = status_text(&status);
    ensure!(
        said.contains("not applied") && said.contains("speech recognition"),
        "{said}"
    );
    Ok(())
}

#[then("the app keeps dictating with the first hotkey")]
fn keeps_first_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let app = app_runtime::run(world);
    ensure!(
        app.hotkey_records(FIRST_HOTKEY),
        "the first hotkey no longer records"
    );
    app.finish_dictation(FIRST_HOTKEY)
}

#[then("a configuration file exists that sets the second hotkey")]
fn file_created(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&settings(world).path())?;
    ensure!(saved.hotkey.key == SECOND_HOTKEY, "{:?}", saved.hotkey.key);
    Ok(())
}

#[then("every other option of that file keeps its default value")]
fn others_default(world: &mut IntegrationWorld) -> Result<()> {
    let mut saved = voicepuncher::config::load(&settings(world).path())?;
    saved.hotkey.key = Config::default().hotkey.key;
    ensure!(Value::try_from(&saved)? == Value::try_from(Config::default())?);
    Ok(())
}

#[then("the settings say the settings apply after the current dictation")]
fn says_waits(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let status = run.wait_for_status(|s| *s == ApplyStatus::WaitsForDictation)?;
    let said = status_text(&status);
    ensure!(said == "Applying after the current dictation…", "{said}");
    ensure!(
        !run.statuses.lock().unwrap().contains(&ApplyStatus::Applied),
        "applied during the recording"
    );
    Ok(())
}

#[then("the minimum press field is marked")]
fn press_marked(world: &mut IntegrationWorld) -> Result<()> {
    let marked = settings(world).marked()?;
    ensure!(marked == "hotkey.min_press_ms", "{marked} is marked");
    Ok(())
}

#[then("the speech recognition address field is marked")]
fn address_marked(world: &mut IntegrationWorld) -> Result<()> {
    let marked = settings(world).marked()?;
    ensure!(marked == "transcription.base_url", "{marked} is marked");
    Ok(())
}

#[then("the settings name the allowed range")]
fn names_range(world: &mut IntegrationWorld) -> Result<()> {
    let refusal = settings(world).refusal.clone().context("not refused")?;
    ensure!(
        refusal.message.contains("between 50 and 1000"),
        "{}",
        refusal.message
    );
    Ok(())
}

#[then("the settings say the address must be on this machine")]
fn says_loopback(world: &mut IntegrationWorld) -> Result<()> {
    let refusal = settings(world).refusal.clone().context("not refused")?;
    ensure!(
        refusal.message.contains("on this Mac"),
        "{}",
        refusal.message
    );
    Ok(())
}

#[then("the configuration file is unchanged")]
fn file_unchanged(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    ensure!(Some(run.file()?) == run.original, "the file changed");
    ensure!(
        run.statuses.lock().unwrap().is_empty(),
        "something was applied"
    );
    Ok(())
}

#[then("the file differs from the original only in the logging level")]
fn differs_only_in_level(world: &mut IntegrationWorld) -> Result<()> {
    let run = settings(world);
    let expected = files::COMMENTED.replace("level = \"info\"", "level = \"debug\"");
    ensure!(run.file()? == expected, "file:\n{}", run.file()?);
    Ok(())
}

#[then("every comment is where it was")]
fn comments_kept(world: &mut IntegrationWorld) -> Result<()> {
    let file = settings(world).file()?;
    let comments = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|line| line.find('#').map(|at| line[at..].to_owned()))
            .collect()
    };
    ensure!(
        comments(&file) == comments(files::COMMENTED),
        "file:\n{file}"
    );
    Ok(())
}

#[then("the refinement examples in the file are unchanged")]
fn examples_kept(world: &mut IntegrationWorld) -> Result<()> {
    let file = settings(world).file()?;
    ensure!(file.contains(files::EXAMPLES_BLOCK), "file:\n{file}");
    Ok(())
}

#[then("the settings ask whether to save, not save or cancel")]
fn asks_on_close(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(
        settings(world).session()?.is_dirty(),
        "closing would ask nothing"
    );
    Ok(())
}

#[then("choosing cancel keeps the settings open with the edit")]
fn cancel_keeps_edit(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("hotkey", "key")?;
    ensure!(shown == SECOND_HOTKEY.human_name(), "shows {shown}");
    Ok(())
}

#[then("the settings close without a question")]
fn closes_quietly(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(!settings(world).session()?.is_dirty(), "closing would ask");
    Ok(())
}

#[then("a note under it names F13 as the hotkey the command line set for this run")]
fn hotkey_note(world: &mut IntegrationWorld) -> Result<()> {
    note_for(world, "hotkey.key", COMMAND_LINE_HOTKEY.human_name())
}

#[then("the refinement switch is on")]
fn refinement_switch_on(world: &mut IntegrationWorld) -> Result<()> {
    let shown = settings(world).shown("refinement", "enabled")?;
    ensure!(shown == "On", "shows {shown}");
    Ok(())
}

#[then("a note under it says the command line turned refinement off for this run")]
fn refinement_note(world: &mut IntegrationWorld) -> Result<()> {
    note_for(world, "refinement.enabled", "Off")
}

fn note_for(world: &mut IntegrationWorld, path: &str, value: &str) -> Result<()> {
    let notes = settings(world).session()?.notes();
    let Some(note) = notes.iter().find(|n| n.field.path() == path) else {
        bail!("no note for {path}: {notes:?}");
    };
    ensure!(
        note.note.contains(value) && note.note.contains("command line"),
        "{}",
        note.note
    );
    Ok(())
}
