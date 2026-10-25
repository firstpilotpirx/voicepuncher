//! Drives the real runtime with an assembler of stand-ins: every pipeline it builds records
//! what its parts were asked to do under the pipeline's generation, a fake keyboard feeds the
//! hotkey binding the runtime holds, and an observer records every runtime state in the same
//! ordered log.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::config::Config;
use voicepuncher::core::application::{PermissionProbe, PermissionsGate};
use voicepuncher::core::domain::{HotKey, HotkeyBinding, Permission, PermissionStatus};
use voicepuncher::runtime::{
    PermissionsWatch, PipelineAssembler, Reloader, Runtime, RuntimeObserver, RuntimeState,
};

use crate::steps::pipeline_config::pipeline_config;
use crate::steps::quiet_voice_control::NoKeys;
use crate::IntegrationWorld;

mod doubles;
mod failures;
pub(crate) mod models;
pub(crate) mod reloads;
mod services;
use crate::steps::menu_bar_presence::doubles::ScriptedFocus;
use doubles::{
    FakeKeyboard, NoticeOverlay, Permissions, RecordingObserver, StandInPaster, StandInSounds,
    StandInSpotter,
};

pub(crate) mod run;
pub(crate) use run::*;
mod lifecycle;
mod outcomes;
pub(crate) use outcomes::*;
mod permissions;
mod stand_ins;
use stand_ins::{Part, StandInAssembler};

const DICTATED: &str = "Meeting moved to Friday.";
const FIRST_HOTKEY: HotKey = HotKey::RightOption;
const SECOND_HOTKEY: HotKey = HotKey::F19;
const THIRD_HOTKEY: HotKey = HotKey::F18;
/// Not on loopback: the configuration does not validate.
const REMOTE_SERVICE: &str = "http://192.0.2.1:8081";
const SILENT_SERVICE_ERROR: &str =
    "speech recognition service is not reachable at http://127.0.0.1:8081";
const FOCUS_POLL: Duration = Duration::from_millis(10);
/// Longest a runtime or pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(2);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
/// How often the runtime checks the permissions in these scenarios.
const PERMISSIONS_CHECK: Duration = Duration::from_millis(20);

type Generation = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Assembled(Generation, HotKey),
    /// The start sound, as its configuration set it.
    Configured(Generation, String),
    RecordingStarted(Generation),
    CancelPlayed(Generation),
    Pasted(Generation),
    ListeningOn(Generation),
    ListeningOff(Generation),
    PipelineDropped(Generation),
    Notice(Generation, String),
    /// The overlay was hidden.
    Hidden(Generation),
    State(RuntimeState),
    ServicesStopped,
}

type Events = Arc<Mutex<Vec<Event>>>;

fn push(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}
