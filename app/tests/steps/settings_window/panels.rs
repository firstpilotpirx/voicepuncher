//! The real settings window built over the settings scenario's configuration file: the line
//! under the voice control switch from the permissions the runtime waits for, a sound played
//! from its Sounds tab and the Models row of its Services tab over a models folder.

use std::cell::Cell;
use std::rc::Rc;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSButton, NSSound};
use objc2_foundation::NSString;
use tempfile::TempDir;
use toml::Value;

use voicepuncher::adapters::primary::modeldownload::macos::mapping::{
    RowStatus, WindowModel as DownloadModel,
};
use voicepuncher::adapters::primary::settings::hints::waiting_note;
use voicepuncher::adapters::primary::settings::macos::field::Tab;
use voicepuncher::adapters::primary::settings::macos::form::Form;
use voicepuncher::adapters::primary::settings::macos::models_row::{
    models_line, ModelsPanel, DOWNLOAD_TITLE, SHOW_TITLE,
};
use voicepuncher::adapters::primary::settings::macos::session::{SessionParts, SettingsSession};
use voicepuncher::adapters::primary::settings::macos::table::FIELDS;
use voicepuncher::adapters::primary::settings::macos::target::Event;
use voicepuncher::adapters::primary::settings::macos::window::SettingsWindow;
use voicepuncher::adapters::secondary::loginitem::macos::MacOsLoginItem;
use voicepuncher::config::Config;
use voicepuncher::core::domain::model_catalog;
use voicepuncher::core::domain::{HotKey, Permission};
use voicepuncher::runtime::models::needs_of;
use voicepuncher::runtime::services::SystemClock;
use voicepuncher::runtime::{CatalogModels, ModelsGate, PermissionsWait, RuntimeState};

use super::{files, settings, text, FakeMicrophones, NoApp};
use crate::steps::app_runtime::run as runtime_run;
use crate::IntegrationWorld;

pub mod devices;
pub mod launch;

/// The sound the user previews.
const PREVIEW_SOUND: &str = "Ping";

/// What a scenario of the settings window shares between its steps.
#[derive(Default)]
pub struct PanelsRun {
    wait: PermissionsWait,
    models: Option<Rc<FolderModels>>,
    form: Option<Rc<Form>>,
    empty_folder: Option<TempDir>,
}

impl std::fmt::Debug for PanelsRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanelsRun")
            .field("waits_for", &self.wait.missing())
            .finish_non_exhaustive()
    }
}

/// The Models row's view of a models folder, counted by the app's catalog gate.
struct FolderModels {
    home: PathBuf,
    config: Config,
    downloads: Cell<usize>,
}

impl ModelsPanel for FolderModels {
    fn count(&self) -> (usize, usize) {
        let needed = model_catalog::needed(needs_of(&self.config)).len();
        let missing = CatalogModels::new(&self.home, HashMap::new()).missing(&self.config);
        (needed - missing.len(), needed)
    }

    fn folder(&self) -> PathBuf {
        self.config.services.models_dir.clone()
    }

    fn open_download(&self) {
        self.downloads.set(self.downloads.get() + 1);
    }
}

pub(super) fn panels(world: &mut IntegrationWorld) -> &mut PanelsRun {
    world
        .extra
        .ui
        .settings_panels
        .get_or_insert_with(PanelsRun::default)
}

fn form(world: &mut IntegrationWorld) -> Result<Rc<Form>> {
    panels(world)
        .form
        .clone()
        .context("the settings window was not opened")
}

/// Builds the settings window over the scenario's file, as the menu bar opens it.
fn open_window(world: &mut IntegrationWorld, tab: Tab) -> Result<()> {
    let mtm = MainThreadMarker::new().context("the harness runs its steps on the main thread")?;
    let _app = NSApplication::sharedApplication(mtm);
    let run = settings(world);
    if !run.path().exists() {
        run.write(files::RIGHT_COMMAND);
    }
    let (path, home) = (run.path(), run.folder.path().to_path_buf());
    let logs = home.join("logs");
    let opener = move || {
        let parts = SessionParts {
            overlay: Box::new(|_: &mut Config| {}),
            applier: Box::new(NoApp),
            devices: Box::new(FakeMicrophones(Vec::new())),
        };
        SettingsSession::open(&path, &home, parts)
    };
    let panel = panels(world);
    let wait = panel.wait.clone();
    let mut window = SettingsWindow::new(
        Box::new(opener),
        logs,
        Arc::new(MacOsLoginItem::of_this_process()),
    )
    .with_permissions_wait(Box::new(move || wait.missing()));
    if let Some(models) = &panel.models {
        window = window.with_models(Rc::clone(models) as Rc<dyn ModelsPanel>);
    }
    let form = window.prepare(mtm)?;
    let place = Tab::ALL
        .iter()
        .position(|t| *t == tab)
        .context("no such tab")?;
    form.select_tab(place);
    panel.form = Some(form);
    Ok(())
}

fn tell(world: &mut IntegrationWorld, state: &RuntimeState) {
    panels(world).wait.observer().state_changed(state);
}

fn waiting_for_accessibility() -> RuntimeState {
    RuntimeState::WaitingForPermissions {
        missing: vec![Permission::Accessibility],
        revoked: vec![],
    }
}

/// The lines under the General tab's switches.
fn general_notes(world: &mut IntegrationWorld) -> Result<Vec<String>> {
    let form = form(world)?;
    let general = form.tabs.iter().find(|views| views.tab == Tab::General);
    let general = general.context("no General tab")?;
    Ok(general
        .notes
        .iter()
        .map(|note| note.stringValue().to_string())
        .collect())
}

#[given("voice control is switched on in the configuration file")]
fn voice_control_on(world: &mut IntegrationWorld) {
    settings(world).write("[voice_control]\nenabled = true\n");
}

#[given("the runtime is waiting for Accessibility")]
fn waiting_for_permission(world: &mut IntegrationWorld) {
    tell(world, &waiting_for_accessibility());
}

#[given("the runtime waited for Accessibility and is Ready now")]
fn ready_after_wait(world: &mut IntegrationWorld) {
    tell(world, &waiting_for_accessibility());
    let ready = RuntimeState::Ready {
        hotkey: HotKey::RightOption,
        recording_mode: RecordingMode::Toggle,
        voice_control: true,
    };
    tell(world, &ready);
}

#[given("every needed model file is in the models folder")]
fn every_model_in_folder(world: &mut IntegrationWorld) -> Result<()> {
    let run = runtime_run(world);
    run.manage_with_models(&[]);
    let home = run
        .models_home()
        .context("the runtime has no models folder")?;
    let config = run.config().clone();
    let models = FolderModels { home, config, downloads: Cell::new(0) };
    panels(world).models = Some(Rc::new(models));
    Ok(())
}

#[when("the user opens the General tab of the settings")]
fn open_general(world: &mut IntegrationWorld) -> Result<()> {
    open_window(world, Tab::General)
}

#[when("the user opens the Services tab")]
fn open_services(world: &mut IntegrationWorld) -> Result<()> {
    open_window(world, Tab::Services)
}

#[when("the user picks a system sound for the stop sound and presses its play button")]
fn play_stop_sound(world: &mut IntegrationWorld) -> Result<()> {
    open_window(world, Tab::Sounds)?;
    let form = form(world)?;
    let tag = FIELDS.iter().position(|f| f.path() == "sounds.stop_sound");
    let tag = tag.context("the stop sound has no field")?;
    form.controls[tag].show(&text(PREVIEW_SOUND));
    form.handle(Event::Play(isize::try_from(tag)?));
    Ok(())
}

#[when("the user picks an empty models folder in the Services tab and saves")]
fn save_empty_models_folder(world: &mut IntegrationWorld) -> Result<()> {
    let config = runtime_run(world).config().clone();
    settings(world).write(&toml::to_string(&config)?);
    super::open_settings(world)?;
    let empty = tempfile::tempdir()?;
    let folder = Value::String(empty.path().display().to_string());
    panels(world).empty_folder = Some(empty);
    let run = settings(world);
    run.edit("services", "models_dir", folder)?;
    run.save()
}

#[then("a line under the voice control switch names Accessibility and says dictation and voice control start once it is granted")]
fn waiting_line_shown(world: &mut IntegrationWorld) -> Result<()> {
    let notes = general_notes(world)?;
    let wanted = waiting_note(&[Permission::Accessibility]).context("no waiting line")?;
    let switch = FIELDS.iter().find(|f| f.path() == "voice_control.enabled");
    if switch.map(|f| f.tab) != Some(Tab::General) {
        bail!("the voice control switch is not on the General tab");
    }
    if notes != [wanted.clone()] || !wanted.contains("once it is granted") {
        bail!("the General tab notes {notes:?}");
    }
    Ok(())
}

#[then("no waiting line is shown under the voice control switch")]
fn no_waiting_line(world: &mut IntegrationWorld) -> Result<()> {
    let notes = general_notes(world)?;
    if notes.iter().any(|note| note.starts_with("Waiting for")) {
        bail!("the General tab notes {notes:?}");
    }
    Ok(())
}

#[then("that sound plays")]
fn sound_plays(_world: &mut IntegrationWorld) -> Result<()> {
    let sound = NSSound::soundNamed(&NSString::from_str(PREVIEW_SOUND));
    let sound = sound.with_context(|| format!("no system sound is named {PREVIEW_SOUND}"))?;
    if !sound.isPlaying() {
        bail!("{PREVIEW_SOUND} does not play");
    }
    sound.stop();
    Ok(())
}

#[then("it shows the models folder and reads that every model is present")]
fn models_all_present(world: &mut IntegrationWorld) -> Result<()> {
    let form = form(world)?;
    let models = panels(world).models.clone().context("no models folder")?;
    let (present, total) = models.count();
    let line = form.models.line.stringValue().to_string();
    if form.models.view.isHidden() || present != total || line != models_line(total, total) {
        bail!("the Models row reads {line:?}");
    }
    let folder = FIELDS.iter().find(|f| f.path() == "services.models_dir");
    if folder.map(|f| f.tab) != Some(Tab::Services) {
        bail!("the models folder is not on the Services tab");
    }
    Ok(())
}

#[then("it offers Download models… and Show in Finder")]
fn offers_download_and_show(world: &mut IntegrationWorld) -> Result<()> {
    let form = form(world)?;
    let titles: Vec<String> = form
        .models
        .view
        .subviews()
        .iter()
        .filter_map(|view| {
            view.downcast_ref::<NSButton>()
                .map(|b| b.title().to_string())
        })
        .collect();
    if titles != [DOWNLOAD_TITLE, SHOW_TITLE] {
        bail!("the Models row offers {titles:?}");
    }
    form.handle(Event::DownloadModels);
    let models = panels(world).models.clone().context("no models folder")?;
    if models.downloads.get() != 1 {
        bail!("Download models… did not open the Download window");
    }
    Ok(())
}

#[then("the runtime waits for models and the Download window lists the files as Missing")]
fn waits_with_missing_files(world: &mut IntegrationWorld) -> Result<()> {
    let run = runtime_run(world);
    run.waits_for_models()?;
    let home = run
        .models_home()
        .context("the runtime has no models folder")?;
    let needs = needs_of(run.config());
    let folder = panels(world)
        .empty_folder
        .as_ref()
        .context("no folder was picked")?;
    let folder = folder.path().to_path_buf();
    let download = CatalogModels::new(&home, HashMap::new()).download(
        folder.clone(),
        needs,
        Arc::new(SystemClock),
    );
    let window = DownloadModel::new(&download.check(), &folder);
    let statuses: Vec<_> = window.rows.iter().map(|row| row.status.clone()).collect();
    if statuses.is_empty() || statuses.iter().any(|status| *status != RowStatus::Missing) {
        bail!("the Download window lists {statuses:?}");
    }
    Ok(())
}
