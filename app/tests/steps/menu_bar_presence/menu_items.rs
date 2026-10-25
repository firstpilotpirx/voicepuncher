//! The menu's waits and its windows: the runtime states the status item's observer is told,
//! and Permissions…, Download models… and Open Logs Folder chosen on the menu's actions, whose
//! windows and file browser are recorded instead of shown.

use std::cell::RefCell;
use std::rc::Rc;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::adapters::primary::menubar::macos::actions::{MacosMenuActions, MenuActions};
use voicepuncher::adapters::primary::menubar::macos::mapping::IconState;
use voicepuncher::adapters::primary::modeldownload::macos::mapping::{
    RowStatus, WindowModel as DownloadModel,
};
use voicepuncher::adapters::primary::permissions::macos::mapping::{
    badge, Shown, WindowModel as PermissionsModel,
};
use voicepuncher::config::Config;
use voicepuncher::core::application::permission_probe::PermissionProbe;
use voicepuncher::core::application::PermissionsGate;
use voicepuncher::core::domain::model_catalog;
use voicepuncher::core::domain::{Permission, PermissionStatus};
use voicepuncher::runtime::models::needs_of;
use voicepuncher::runtime::services::SystemClock;
use voicepuncher::runtime::{CatalogModels, RuntimeObserver, RuntimeState, ServicesView};

use super::{run, services::ViewDouble, MenuBarRun, SettingsOpens};
use crate::steps::app_runtime::models::DICTATION_MODEL;
use crate::steps::model_download::files::place;
use crate::IntegrationWorld;

mod dock;

const PERMISSIONS_WINDOW: &str = "Permissions";
const DOWNLOAD_WINDOW: &str = "Download";

/// The windows the menu brought to the front, in order.
type Opened = Rc<RefCell<Vec<&'static str>>>;

/// What a scenario of the menu's waits and windows shares between its steps.
#[derive(Default)]
pub struct MenuItemsRun {
    opened: Opened,
    permissions: Rc<RefCell<Option<PermissionsModel>>>,
    download: Rc<RefCell<Option<DownloadModel>>>,
    /// A home whose models folder holds every needed file, when the app manages its services.
    models: Option<TempDir>,
    /// The folders the file browser was asked to show, each with whether it existed then.
    shown: Rc<RefCell<Vec<(PathBuf, bool)>>>,
    scratch: Option<TempDir>,
    actions: Option<MacosMenuActions>,
    /// The status item and the overlay of a running app, kept while the scenario looks.
    pub(super) running: Option<dock::RunningApp>,
}

impl std::fmt::Debug for MenuItemsRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItemsRun")
            .field("opened", &self.opened)
            .field("shown", &self.shown)
            .finish_non_exhaustive()
    }
}

impl MenuItemsRun {
    fn logs_dir(&mut self) -> PathBuf {
        let scratch = self
            .scratch
            .get_or_insert_with(|| tempfile::tempdir().expect("a temporary folder"));
        scratch.path().join("logs")
    }

    /// The menu's actions over the scenario's runtime, with windows that record what they show.
    fn actions(&mut self, bar: &mut MenuBarRun) -> &MacosMenuActions {
        if self.actions.is_none() {
            let runtime = bar
                .runtime
                .take()
                .unwrap_or_else(|| bar.runtime_parts(None));
            let settings = Box::new(SettingsOpens(Arc::default()));
            let mut actions = MacosMenuActions::new(runtime, settings, self.logs_dir())
                .with_permissions_window(self.permissions_window())
                .with_file_browser(self.file_browser());
            if let Some(home) = &self.models {
                actions = actions
                    .with_services(Arc::new(ViewDouble::default()) as Arc<dyn ServicesView>)
                    .with_download_window(download_window(
                        home.path(),
                        &self.opened,
                        &self.download,
                    ));
            }
            self.actions = Some(actions);
        }
        self.actions.as_ref().expect("the actions were just made")
    }

    fn permissions_window(&self) -> Box<dyn Fn()> {
        let opened = Rc::clone(&self.opened);
        let shown = Rc::clone(&self.permissions);
        Box::new(move || {
            opened.borrow_mut().push(PERMISSIONS_WINDOW);
            let report = PermissionsGate::new(Box::new(EveryPermissionGranted)).report();
            *shown.borrow_mut() = Some(PermissionsModel::new(&report, &[], &[]));
        })
    }

    fn file_browser(&self) -> Box<dyn Fn(&Path) -> bool> {
        let shown = Rc::clone(&self.shown);
        Box::new(move |folder: &Path| {
            shown
                .borrow_mut()
                .push((folder.to_path_buf(), folder.is_dir()));
            true
        })
    }
}

/// The Download window as it opens: a fresh check of the models folder of `home`.
fn download_window(
    home: &Path,
    opened: &Opened,
    shown: &Rc<RefCell<Option<DownloadModel>>>,
) -> Box<dyn Fn()> {
    let (home, opened, shown) = (home.to_path_buf(), Rc::clone(opened), Rc::clone(shown));
    Box::new(move || {
        opened.borrow_mut().push(DOWNLOAD_WINDOW);
        let folder = home.join("models");
        let download = CatalogModels::new(&home, HashMap::new()).download(
            folder.clone(),
            needs_of(&Config::default()),
            Arc::new(SystemClock),
        );
        *shown.borrow_mut() = Some(DownloadModel::new(&download.check(), &folder));
    })
}

/// Every permission granted, as System Settings holds them.
struct EveryPermissionGranted;

impl PermissionProbe for EveryPermissionGranted {
    fn status(&self, _permission: Permission) -> PermissionStatus {
        PermissionStatus::Granted
    }

    fn request(&self, _permission: Permission) {}

    fn settings_url(&self, _permission: Permission) -> String {
        String::new()
    }

    fn responsible_app(&self) -> Option<String> {
        None
    }
}

/// A home whose models folder holds every file the default configuration needs.
fn every_model_present() -> TempDir {
    let home = tempfile::tempdir().expect("a temporary home");
    let folder = home.path().join("models");
    for file in model_catalog::needed(needs_of(&Config::default())) {
        place(&folder, &file, file.size_bytes).expect("a model file");
    }
    home
}

pub(super) fn items(world: &mut IntegrationWorld) -> &mut MenuItemsRun {
    world
        .extra
        .ui
        .menu_items
        .get_or_insert_with(MenuItemsRun::default)
}

/// The scenario's menu actions and the status item's run, borrowed together.
fn with_actions(world: &mut IntegrationWorld, act: impl FnOnce(&MacosMenuActions)) {
    let bar = world.app.menu_bar.get_or_insert_with(MenuBarRun::default);
    let items = world
        .extra
        .ui
        .menu_items
        .get_or_insert_with(MenuItemsRun::default);
    act(items.actions(bar));
}

fn tell(world: &mut IntegrationWorld, state: &RuntimeState) {
    run(world).cell.observer().state_changed(state);
}

#[given(regex = r"^the runtime waits for (Accessibility|Accessibility and Input Monitoring)$")]
fn waits_for_permissions(world: &mut IntegrationWorld, names: String) {
    let missing = Permission::ALL
        .into_iter()
        .filter(|permission| names.contains(permission.name()))
        .collect();
    tell(
        world,
        &RuntimeState::WaitingForPermissions { missing, revoked: vec![] },
    );
}

#[given("the dictation model is missing when the app launches")]
fn dictation_model_missing(world: &mut IntegrationWorld) {
    let folder = std::env::temp_dir().join("models");
    let missing = vec![DICTATION_MODEL];
    tell(world, &RuntimeState::WaitingForModels { folder, missing });
}

#[given("the runtime is ready with every model present and the services the app manages")]
fn ready_with_models(world: &mut IntegrationWorld) -> Result<()> {
    items(world).models = Some(every_model_present());
    run(world).start_ready()
}

#[given("the logs folder exists")]
fn logs_folder_exists(world: &mut IntegrationWorld) -> Result<()> {
    let logs = items(world).logs_dir();
    std::fs::create_dir_all(&logs).context("the logs folder")
}

#[given("the logs folder does not exist")]
fn logs_folder_missing(world: &mut IntegrationWorld) {
    items(world).logs_dir();
}

#[when(regex = r#"^the user chooses "(.+)" in the menu$"#)]
fn chooses_status_line(world: &mut IntegrationWorld, line: String) {
    let menu = run(world).cell.menu();
    if menu.status_line == line && menu.status_opens_permissions {
        with_actions(world, MenuActions::open_permissions);
    }
}

#[when("the user chooses Permissions… in the menu")]
fn chooses_permissions(world: &mut IntegrationWorld) {
    with_actions(world, MenuActions::open_permissions);
}

#[when("the user chooses Download models… in the menu")]
fn chooses_download_models(world: &mut IntegrationWorld) {
    with_actions(world, |actions| {
        // the menu offers the item only while the app manages its services
        if actions.services().is_some() {
            actions.open_download_models();
        }
    });
}

#[when("the user chooses Open Logs Folder")]
fn chooses_open_logs(world: &mut IntegrationWorld) {
    with_actions(world, MenuActions::open_logs);
}

#[then("the status item shows the missing-permission icon, not the waiting icon")]
fn shows_missing_permission(world: &mut IntegrationWorld) -> Result<()> {
    run(world).icon_is(IconState::MissingPermission)?;
    if IconState::MissingPermission.picture() == IconState::Waiting.picture() {
        bail!("the missing-permission icon looks like the waiting icon");
    }
    Ok(())
}

fn opened_once(world: &mut IntegrationWorld, window: &str) -> Result<()> {
    let opened = items(world).opened.borrow().clone();
    if opened != [window] {
        bail!("the menu opened {opened:?}");
    }
    Ok(())
}

#[then("the Permissions window comes to the front")]
fn permissions_in_front(world: &mut IntegrationWorld) -> Result<()> {
    opened_once(world, PERMISSIONS_WINDOW)
}

#[then("the Permissions window opens with all three rows marked Granted")]
fn permissions_all_granted(world: &mut IntegrationWorld) -> Result<()> {
    opened_once(world, PERMISSIONS_WINDOW)?;
    let model = items(world).permissions.borrow().clone();
    let model = model.context("the Permissions window showed nothing")?;
    let marks: Vec<&str> = model.rows.iter().map(|row| badge(row.shown).0).collect();
    let granted = Shown::Status(PermissionStatus::Granted);
    if model.rows.len() != 3 || model.rows.iter().any(|row| row.shown != granted) {
        bail!("the rows are marked {marks:?}");
    }
    Ok(())
}

#[then("the Download window opens with every row marked Present and Done enabled")]
fn download_all_present(world: &mut IntegrationWorld) -> Result<()> {
    opened_once(world, DOWNLOAD_WINDOW)?;
    let model = items(world).download.borrow().clone();
    let model = model.context("the Download window showed nothing")?;
    let statuses: Vec<_> = model.rows.iter().map(|row| row.status.clone()).collect();
    if statuses.is_empty() || statuses.iter().any(|status| *status != RowStatus::Present) {
        bail!("the rows are marked {statuses:?}");
    }
    if !model.done_enabled() {
        bail!("Done is disabled");
    }
    Ok(())
}

fn shows_logs_folder(world: &mut IntegrationWorld) -> Result<()> {
    let items = items(world);
    let logs = items.logs_dir();
    let shown = items.shown.borrow().clone();
    if shown != [(logs.clone(), true)] {
        bail!(
            "the file browser was asked to show {shown:?}, not {}",
            logs.display()
        );
    }
    Ok(())
}

#[then("the file browser shows the logs folder")]
fn file_browser_shows_logs(world: &mut IntegrationWorld) -> Result<()> {
    shows_logs_folder(world)
}

#[then("the logs folder exists and the file browser shows it")]
fn logs_folder_created_and_shown(world: &mut IntegrationWorld) -> Result<()> {
    let logs = items(world).logs_dir();
    if !logs.is_dir() {
        bail!("{} does not exist", logs.display());
    }
    shows_logs_folder(world)
}
