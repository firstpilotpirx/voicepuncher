//! Drives the login switch and the one-time offer over a login item double and a real
//! configuration file in a temporary folder; the system's login items are never touched.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::primary::settings::macos::login::{
    refusal_text, LoginOffer, LoginSwitch, LoginSwitchView, OfferAnswer, APPROVAL_NOTE,
    UNAVAILABLE_NOTE,
};
use voicepuncher::adapters::primary::settings::macos::session::{
    ConfigApplier, SessionParts, SettingsSession, StatusListener,
};
use voicepuncher::adapters::secondary::loginitem::macos::launch::{launch_line, LAUNCHED_AT_LOGIN};
use voicepuncher::config::Config;
use voicepuncher::core::application::{InputDevices, LoginItem, LoginItemStatus};

use crate::IntegrationWorld;

/// The open-application Apple event, and its property when the login item started the app.
const OPEN_APPLICATION: u32 = u32::from_be_bytes(*b"oapp");
const LAUNCHED_AS_LOGIN_ITEM: u32 = u32::from_be_bytes(*b"lgit");
const REFUSAL: &str = "The operation couldn't be completed. Operation not permitted";
const SETTINGS: &str = "# kept by hand\n[hotkey]\nkey = \"RightCommand\"\n";
const ASKED_BEFORE: &str =
    "[hotkey]\nkey = \"RightCommand\"\n\n[app]\nlaunch_at_login_asked = true\n";

/// The system's login item of the app, as a double.
#[derive(Debug)]
pub struct LoginItemDouble {
    status: Mutex<LoginItemStatus>,
    refuses: Mutex<bool>,
    changes: Mutex<usize>,
}

impl LoginItemDouble {
    fn new(status: LoginItemStatus) -> Self {
        Self { status: Mutex::new(status), refuses: Mutex::new(false), changes: Mutex::new(0) }
    }

    fn set(&self, status: LoginItemStatus) {
        *self.status.lock().unwrap() = status;
    }

    fn change(&self, to: LoginItemStatus) -> Result<(), String> {
        *self.changes.lock().unwrap() += 1;
        if *self.refuses.lock().unwrap() {
            return Err(REFUSAL.to_owned());
        }
        self.set(to);
        Ok(())
    }
}

impl LoginItem for LoginItemDouble {
    fn status(&self) -> LoginItemStatus {
        *self.status.lock().unwrap()
    }

    fn register(&self) -> Result<(), String> {
        self.change(LoginItemStatus::Enabled)
    }

    fn unregister(&self) -> Result<(), String> {
        self.change(LoginItemStatus::NotRegistered)
    }
}

/// What a launch-at-login scenario shares between its steps.
pub struct LoginRun {
    folder: tempfile::TempDir,
    item: Arc<LoginItemDouble>,
    view: Option<LoginSwitchView>,
    status_line: Option<String>,
    /// The offer of this run; a relaunch starts a new one.
    offer: Option<LoginOffer>,
    asked: Option<bool>,
    /// The log line of the last launch; `None` before a launch or when it logs none.
    launch_line: Option<&'static str>,
}

impl std::fmt::Debug for LoginRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginRun")
            .field("item", &self.item)
            .field("view", &self.view)
            .field("asked", &self.asked)
            .finish()
    }
}

impl Default for LoginRun {
    fn default() -> Self {
        let run = Self {
            folder: tempfile::tempdir().expect("a temporary folder"),
            item: Arc::new(LoginItemDouble::new(LoginItemStatus::NotRegistered)),
            view: None,
            status_line: None,
            offer: None,
            asked: None,
            launch_line: None,
        };
        std::fs::write(run.path(), SETTINGS).expect("the configuration file is written");
        run
    }
}

impl LoginRun {
    pub(crate) fn path(&self) -> PathBuf {
        self.folder.path().join("config.toml")
    }

    pub(crate) fn item(&self) -> Arc<LoginItemDouble> {
        Arc::clone(&self.item)
    }

    fn switch(&self) -> LoginSwitch {
        LoginSwitch::new(self.item())
    }

    fn reach_ready(&mut self) {
        let offer = self.offer.get_or_insert_with(|| {
            LoginOffer::new(
                Arc::clone(&self.item) as _,
                self.folder.path().join("config.toml"),
            )
        });
        self.asked = Some(offer.due());
    }

    fn answer(&mut self, answer: OfferAnswer) -> Result<()> {
        let offer = self.offer.as_ref().context("the app never reached Ready")?;
        ensure!(self.asked == Some(true), "the app did not ask");
        offer.answer(answer).map_err(anyhow::Error::msg)
    }

    fn view(&self) -> Result<&LoginSwitchView> {
        self.view.as_ref().context("the General tab was not opened")
    }
}

pub(crate) fn login(world: &mut IntegrationWorld) -> &mut LoginRun {
    world.login.get_or_insert_with(LoginRun::default)
}

fn status(world: &mut IntegrationWorld) -> LoginItemStatus {
    login(world).item.status()
}

#[given("the app is registered as a login item")]
fn registered(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::Enabled);
}

#[given("the app is not registered as a login item")]
fn not_registered(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::NotRegistered);
}

#[given("the app's login item waits for the user's approval")]
fn waits_for_approval(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::RequiresApproval);
}

#[given("the app runs from its build folder")]
fn build_folder(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::Unavailable);
}

#[given("the system refuses to register it")]
fn refuses(world: &mut IntegrationWorld) {
    *login(world).item.refuses.lock().unwrap() = true;
}

#[given("the user removed the app from the login items in System Settings")]
fn removed_in_system_settings(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::NotRegistered);
}

#[given("the user opens the General tab")]
#[when("the user opens the General tab")]
#[when("the settings window comes to the front again")]
fn open_general(world: &mut IntegrationWorld) {
    let run = login(world);
    run.view = Some(run.switch().view());
}

#[when("the user turns the login switch on")]
fn turn_on(world: &mut IntegrationWorld) {
    turn(world, true);
}

#[when("the user turns the login switch off")]
fn turn_off(world: &mut IntegrationWorld) {
    turn(world, false);
}

fn turn(world: &mut IntegrationWorld, on: bool) {
    let run = login(world);
    let turned = run.switch().turn(on);
    run.status_line = turned.refusal.as_deref().map(refusal_text);
    run.view = Some(turned.view);
}

#[then("the login switch is on")]
fn switch_on(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(login(world).view()?.on, "the switch is off");
    Ok(())
}

#[then("the login switch is off")]
fn switch_off(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(!login(world).view()?.on, "the switch is on");
    Ok(())
}

#[then("no approval note is shown")]
fn no_note(world: &mut IntegrationWorld) -> Result<()> {
    let view = login(world).view()?;
    ensure!(view.note.is_none() && !view.approval_button, "{view:?}");
    Ok(())
}

#[then("a note says it waits for approval in System Settings")]
fn approval_note(world: &mut IntegrationWorld) -> Result<()> {
    let note = login(world).view()?.note;
    ensure!(note == Some(APPROVAL_NOTE), "{note:?}");
    Ok(())
}

#[then("a button opens the Login Items settings")]
fn approval_button(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(login(world).view()?.approval_button, "no button");
    Ok(())
}

#[then(regex = r"^the login switch is (enabled|disabled)$")]
fn enabled(world: &mut IntegrationWorld, state: String) -> Result<()> {
    let enabled = login(world).view()?.enabled;
    ensure!(enabled == (state == "enabled"), "the switch is not {state}");
    Ok(())
}

#[then("a note says it is available when voicepuncher runs as an installed app")]
fn unavailable_note(world: &mut IntegrationWorld) -> Result<()> {
    let note = login(world).view()?.note;
    ensure!(note == Some(UNAVAILABLE_NOTE), "{note:?}");
    Ok(())
}

#[then("nothing was registered or unregistered")]
fn nothing_changed(world: &mut IntegrationWorld) -> Result<()> {
    let changes = *login(world).item.changes.lock().unwrap();
    ensure!(changes == 0, "{changes} changes asked of the system");
    Ok(())
}

#[then("the app is registered as a login item")]
fn is_registered(world: &mut IntegrationWorld) -> Result<()> {
    let status = status(world);
    ensure!(status == LoginItemStatus::Enabled, "{status:?}");
    Ok(())
}

#[then("the app is not registered as a login item")]
fn is_not_registered(world: &mut IntegrationWorld) -> Result<()> {
    let status = status(world);
    ensure!(status == LoginItemStatus::NotRegistered, "{status:?}");
    Ok(())
}

#[then("the configuration file is left as it was")]
fn file_unchanged(world: &mut IntegrationWorld) -> Result<()> {
    let file = std::fs::read_to_string(login(world).path())?;
    ensure!(file == SETTINGS, "the file became {file:?}");
    Ok(())
}

#[then("closing the settings window asks nothing")]
fn closing_asks_nothing(world: &mut IntegrationWorld) -> Result<()> {
    let path = login(world).path();
    let session = SettingsSession::open(&path, &PathBuf::from("/"), session_parts())?;
    ensure!(!session.is_dirty(), "the settings have unsaved edits");
    Ok(())
}

#[then("the status line shows the system's reason")]
fn reason_shown(world: &mut IntegrationWorld) -> Result<()> {
    let line = login(world).status_line.clone().unwrap_or_default();
    ensure!(line.contains(REFUSAL), "the status line says {line:?}");
    Ok(())
}

#[given("the installed app has never asked about starting at login")]
fn never_asked(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::NotRegistered);
}

#[given("the configuration file records that the question was asked")]
fn asked_before(world: &mut IntegrationWorld) {
    let run = login(world);
    run.item.set(LoginItemStatus::NotRegistered);
    std::fs::write(run.path(), ASKED_BEFORE).expect("the configuration file is written");
}

#[given("the app reached Ready and asked whether to start at login")]
#[when("the app reaches Ready")]
#[when("a saved setting restarts the pipeline and the app reaches Ready again")]
fn reaches_ready(world: &mut IntegrationWorld) {
    login(world).reach_ready();
}

#[when("the app reaches Ready after a relaunch")]
fn reaches_ready_after_relaunch(world: &mut IntegrationWorld) {
    let run = login(world);
    run.offer = None;
    run.reach_ready();
}

#[when("the user answers Not Now")]
fn answers_not_now(world: &mut IntegrationWorld) -> Result<()> {
    login(world).answer(OfferAnswer::NotNow)
}

#[then("the app asks whether to start at login")]
fn asks(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(login(world).asked == Some(true), "the app did not ask");
    Ok(())
}

#[then("answering Start at Login registers the app as a login item")]
fn start_at_login_registers(world: &mut IntegrationWorld) -> Result<()> {
    login(world).answer(OfferAnswer::StartAtLogin)?;
    let status = status(world);
    ensure!(status == LoginItemStatus::Enabled, "{status:?}");
    Ok(())
}

#[then("the configuration file records that the question was asked")]
fn records_asked(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&login(world).path())?;
    let asked = saved.app.launch_at_login_asked;
    ensure!(asked, "the file does not record the question");
    Ok(())
}

#[then("the app asks nothing")]
#[then("the app asks nothing more")]
fn asks_nothing(world: &mut IntegrationWorld) -> Result<()> {
    let asked = login(world).asked;
    ensure!(asked == Some(false), "asked: {asked:?}");
    Ok(())
}

struct NoApplier;

impl ConfigApplier for NoApplier {
    fn apply(&self, _config: Config, _listener: StatusListener) {}
}

struct NoDevices;

impl InputDevices for NoDevices {
    fn names(&self) -> Vec<String> {
        Vec::new()
    }
}

fn session_parts() -> SessionParts {
    SessionParts {
        overlay: Box::new(|_| {}),
        applier: Box::new(NoApplier),
        devices: Box::new(NoDevices),
    }
}

#[when("the system starts the app at login")]
fn started_at_login(world: &mut IntegrationWorld) {
    login(world).launch_line = launch_line(Some(OPEN_APPLICATION), Some(LAUNCHED_AS_LOGIN_ITEM));
}

#[then("the log says the app was launched at login")]
fn logged_at_login(world: &mut IntegrationWorld) -> Result<()> {
    let line = login(world).launch_line;
    ensure!(
        line == Some(LAUNCHED_AT_LOGIN),
        "the launch logged {line:?}"
    );
    Ok(())
}
