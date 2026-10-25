//! Drives the login switch and the one-time offer over a login item double and a real
//! configuration file in a temporary folder; the system's login items are never touched.

use voicepuncher::test_doubles::prelude::*;

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

mod first_launch;
mod switch;
use first_launch::session_parts;

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
            .finish_non_exhaustive()
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
    world.app.login.get_or_insert_with(LoginRun::default)
}

fn status(world: &mut IntegrationWorld) -> LoginItemStatus {
    login(world).item.status()
}
