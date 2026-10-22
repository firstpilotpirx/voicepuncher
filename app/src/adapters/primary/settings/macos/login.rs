//! The login item without `AppKit`: what the General tab's switch shows and what turning it
//! does, and the one-time offer to start at login.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::config::document::ConfigDocument;
use crate::core::application::login_item::{LoginItem, LoginItemStatus};

pub const SWITCH_LABEL: &str = "Start voicepuncher when you log in";
pub const APPROVAL_NOTE: &str = "Waiting for your approval in System Settings";
pub const UNAVAILABLE_NOTE: &str = "Available when voicepuncher runs as an installed app";
pub const OPEN_LOGIN_ITEMS_TITLE: &str = "Open Login Items Settings…";
/// System Settings → General → Login Items.
pub const LOGIN_ITEMS_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.LoginItems-Settings.extension";

pub const OFFER_MESSAGE: &str = "Start voicepuncher when you log in?";
pub const OFFER_INFO: &str = "You can change this any time in Settings → General.";
pub const START_AT_LOGIN_TITLE: &str = "Start at Login";
pub const NOT_NOW_TITLE: &str = "Not Now";
/// Where the configuration file records that the offer was made.
const OFFER_SECTION: &str = "app";
const OFFER_KEY: &str = "launch_at_login_asked";

/// What the login row shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSwitchView {
    pub on: bool,
    pub enabled: bool,
    pub note: Option<&'static str>,
    /// Whether the button to the Login Items settings is shown.
    pub approval_button: bool,
}

/// What turning the switch led to: the row to show, and the system's reason when it
/// refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turned {
    pub view: LoginSwitchView,
    pub refusal: Option<String>,
}

/// The row of `status`.
pub fn shown(status: LoginItemStatus) -> LoginSwitchView {
    let view =
        |on, enabled, note, approval_button| LoginSwitchView { on, enabled, note, approval_button };
    match status {
        LoginItemStatus::NotRegistered => view(false, true, None, false),
        LoginItemStatus::Enabled => view(true, true, None, false),
        LoginItemStatus::RequiresApproval => view(true, true, Some(APPROVAL_NOTE), true),
        LoginItemStatus::Unavailable => view(false, false, Some(UNAVAILABLE_NOTE), false),
    }
}

/// What the status line says when the system refused to change the login item.
pub fn refusal_text(reason: &str) -> String {
    format!("Could not change the login item: {reason}")
}

/// The switch over the app's login item; it acts at once and takes no part in Save.
pub struct LoginSwitch {
    item: Arc<dyn LoginItem>,
}

impl LoginSwitch {
    pub fn new(item: Arc<dyn LoginItem>) -> Self {
        Self { item }
    }

    /// The row as the system's status has it now.
    pub fn view(&self) -> LoginSwitchView {
        shown(self.item.status())
    }

    /// The user turned the switch to `on`: registers or unregisters the app; a refusal
    /// leaves the row as the system has it, the switch back where it was.
    pub fn turn(&self, on: bool) -> Turned {
        let before = self.item.status();
        if before == LoginItemStatus::Unavailable || shown(before).on == on {
            return Turned { view: shown(before), refusal: None };
        }
        let changed = if on {
            self.item.register()
        } else {
            self.item.unregister()
        };
        Turned { view: self.view(), refusal: changed.err() }
    }
}

/// The user's answer to the offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferAnswer {
    StartAtLogin,
    NotNow,
}

/// The offer to start at login, made once: at the first Ready of an installed app that is
/// not registered and whose configuration file does not record the question.
pub struct LoginOffer {
    item: Arc<dyn LoginItem>,
    config_path: PathBuf,
    made: AtomicBool,
}

impl LoginOffer {
    pub fn new(item: Arc<dyn LoginItem>, config_path: PathBuf) -> Self {
        Self { item, config_path, made: AtomicBool::new(false) }
    }

    /// Whether reaching Ready asks now; true at most once per run, so a reload that
    /// reaches Ready again never asks twice.
    pub fn due(&self) -> bool {
        if self.made.load(Ordering::SeqCst) || recorded(&self.config_path) {
            return false;
        }
        self.item.status() == LoginItemStatus::NotRegistered
            && !self.made.swap(true, Ordering::SeqCst)
    }

    /// Registers the app on Start at Login and records either answer; the error says
    /// what failed.
    pub fn answer(&self, answer: OfferAnswer) -> Result<(), String> {
        let registered = match answer {
            OfferAnswer::StartAtLogin => self.item.register(),
            OfferAnswer::NotNow => Ok(()),
        };
        let recorded = record(&self.config_path).map_err(|e| format!("{e:#}"));
        registered.and(recorded)
    }
}

/// Whether the file records the question; a file that cannot be read counts as recorded,
/// so a broken file never brings the question back on every launch.
fn recorded(path: &Path) -> bool {
    ConfigDocument::load(path)
        .and_then(|document| document.to_config())
        .map_or(true, |config| config.app.launch_at_login_asked)
}

fn record(path: &Path) -> anyhow::Result<()> {
    let mut document = ConfigDocument::load(path)?;
    document.set(OFFER_SECTION, OFFER_KEY, toml_edit::value(true));
    document.save(path)
}
