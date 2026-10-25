//! Drives the real runtime with pipelines of stand-ins whose overlay goes through the status
//! item's decorator; the runtime tells the status item's observer its states, and a status
//! item double records every icon it is asked to draw.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::adapters::primary::menubar::macos::actions::{
    MacosMenuActions, MenuActions, SettingsOpener,
};
use voicepuncher::adapters::primary::menubar::macos::indicator::{
    MenuModel, StatusItemCell, StatusItemSurface,
};
use voicepuncher::core::domain::{CommandPlacement, HotKey, HotkeyBinding};
use voicepuncher::runtime::{
    PipelineAssembler, Runtime, RuntimeObserver, RuntimeState, SharedVisualIndicator,
};
use voicepuncher::{adapters::primary::menubar::macos::mapping::IconState, config::Config};

use crate::steps::pipeline_config::pipeline_config;
use crate::steps::quiet_voice_control::{NoKeys, QuietSpotter};
use crate::IntegrationWorld;

pub(crate) mod doubles;
pub mod menu_items;
mod services;
use doubles::{FakeKeyboard, QuietOverlay, StandInAssembler};

mod run;
pub(crate) use run::*;
mod givens;
mod whens;
use whens::{commands_away, SettingsOpens};
mod thens;

const DICTATED: &str = "Meeting moved to Friday.";
const SILENT_SERVICE_ERROR: &str =
    "speech recognition service is not reachable at http://127.0.0.1:8081";
/// How long the status item shows the warning of a failure notice in these scenarios.
const NOTICE_LIFETIME: Duration = Duration::from_millis(400);
/// How early the warning may go, as the timer and the recording of icons may differ.
const NOTICE_SLACK: Duration = Duration::from_millis(50);
const FOCUS_POLL: Duration = Duration::from_millis(10);
const FOCUS_WAIT_SECONDS: u64 = 5;
/// Longest a runtime or pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
const POLL: Duration = Duration::from_millis(5);

/// What a quit does, in the order it does it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QuitEvent {
    RuntimeStopped,
    ProcessEnded(i32),
}

type Icons = Arc<Mutex<Vec<(Instant, IconState)>>>;
pub(crate) type Flag = Arc<Mutex<bool>>;
/// A start held while it assembles: the gate that lets it go on, and the thread it runs on.
type HeldStart = (Sender<bool>, JoinHandle<(Runtime, Result<()>)>);
