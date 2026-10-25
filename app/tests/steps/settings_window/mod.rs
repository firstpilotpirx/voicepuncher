//! Drives a settings session over a real configuration file in a temporary folder; when a
//! scenario runs the app, saved settings reach the app-runtime stand-ins through the real
//! runtime, as the menu bar applies them.

mod files;
pub mod panels;

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use toml::Value;

use voicepuncher::adapters::primary::menubar::macos::applier::RuntimeApplier;
use voicepuncher::adapters::primary::settings::macos::field::{Choices, Field, Tab};
use voicepuncher::adapters::primary::settings::macos::session::{
    status_text, ApplyStatus, ConfigApplier, Refusal, SessionParts, SettingsSession, StatusListener,
};
use voicepuncher::adapters::primary::settings::macos::table::{find, FIELDS};
use voicepuncher::adapters::primary::settings::macos::values::display;
use voicepuncher::config::Config;
use voicepuncher::core::application::InputDevices;
use voicepuncher::core::domain::HotKey;

use crate::{steps::app_runtime, IntegrationWorld};

mod run;
pub(crate) use run::*;
mod givens;
use givens::open_settings;
mod applied;
mod edits;
mod file;

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
