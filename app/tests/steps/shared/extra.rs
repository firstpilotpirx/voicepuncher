//! The world's room for the scenarios added after `IntegrationWorld` reached its field budget:
//! one slot per run kept between the steps of a scenario, grouped by area.

use crate::steps::app_logging::panic::PanicRun;
use crate::steps::app_runtime::reloads::{HangUpRun, RebindRun};
use crate::steps::dictation_archive::limit::LimitRun as ArchiveLimitRun;
use crate::steps::file_transcription::CommandLineRun;
use crate::steps::menu_bar_presence::menu_items::MenuItemsRun;
use crate::steps::model_download::files::ModelFilesRun;
use crate::steps::permissions_gate::window::WindowView;
use crate::steps::push_to_talk::hold::HoldRun;
use crate::steps::push_to_talk::terminal::LaunchRun;
use crate::steps::settings_window::panels::devices::MicrophoneRun;
use crate::steps::settings_window::panels::launch::FirstLaunchRun;
use crate::steps::settings_window::panels::PanelsRun;
use crate::steps::text_insertion::clipboard::ClipboardRun;
use crate::steps::text_insertion::filler_sounds::FillerRun;
use crate::steps::visual_feedback::{limit::LimitRun, overlay::OverlayRun, pointer::PointerRun};

#[derive(Debug, Default)]
pub struct Extra {
    /// The text a speech recognition scenario saw pasted.
    pub pasted: Option<String>,
    pub ptt: PttRuns,
    pub view: ViewRuns,
    pub runtime: RuntimeRuns,
    pub insertion: InsertionRuns,
    pub ui: UiRuns,
    pub speech: SpeechRuns,
    /// The local-services scenario's runtime, catalog, copies or command spotting client.
    pub local_services: Option<crate::steps::local_services::LocalRun>,
}

/// The push-to-talk scenarios' runs.
#[derive(Debug, Default)]
pub struct PttRuns {
    /// The pipeline in Hold mode.
    pub hold: Option<HoldRun>,
    /// The first launch and key classification.
    pub launch: Option<LaunchRun>,
}

/// The runs of the scenarios on the real overlay, dictation sign and recording buffer.
#[derive(Debug, Default)]
pub struct ViewRuns {
    /// The real overlay on the main thread.
    pub overlay: Option<OverlayRun>,
    /// The real dictation sign on the main thread.
    pub pointer: Option<PointerRun>,
    /// The recording-limit scenario's recording buffer.
    pub limit: Option<LimitRun>,
}

/// The app-runtime, app-logging and dictation-archive scenarios' runs.
#[derive(Debug, Default)]
pub struct RuntimeRuns {
    /// The configuration file and the thread that reloads it.
    pub hang_up: Option<HangUpRun>,
    /// The hotkey binding and what the listener signalled.
    pub rebind: Option<RebindRun>,
    /// The panicking thread.
    pub panic: Option<PanicRun>,
    /// The archive at its size limit.
    pub archive_limit: Option<ArchiveLimitRun>,
}

/// The text-insertion and file-transcription scenarios' runs.
#[derive(Debug, Default)]
pub struct InsertionRuns {
    /// Where the focus lines begin in the shared log.
    pub focus_log_mark: Option<usize>,
    /// The pasteboard and paster.
    pub clipboard: Option<ClipboardRun>,
    /// The transcript and what was pasted.
    pub filler: Option<FillerRun>,
    /// The command line run.
    pub command_line: Option<CommandLineRun>,
}

/// The menu bar, permissions, settings and model download scenarios' runs.
#[derive(Debug, Default)]
pub struct UiRuns {
    /// The menu-bar-presence scenario's waits and the windows its menu opened.
    pub menu_items: Option<MenuItemsRun>,
    /// The model-download scenario's presence check and headless download.
    pub model_files: Option<ModelFilesRun>,
    /// The permissions-gate scenario's Permissions window and what its buttons did.
    pub permissions_window: Option<WindowView>,
    /// The settings-window scenario's first launch and its read of the default file.
    pub first_launch: Option<FirstLaunchRun>,
    /// The settings-window scenario's real window and what it was built over.
    pub settings_panels: Option<PanelsRun>,
    /// The settings-window scenario's configured microphone and the device it opened.
    pub microphone: Option<MicrophoneRun>,
}

/// The speech recognition and voice control scenarios' runs.
#[derive(Debug, Default)]
pub struct SpeechRuns {
    /// The speech recognition client's scenario against a stand-in service.
    pub service: Option<crate::steps::speech_recognition::client::ServiceRun>,
    /// The speech recognition scenario's recording on a stand-in microphone.
    pub recording: Option<crate::steps::speech_recognition::recording::RecordingRun>,
    /// The voice-control scenario's kept microphone or command spotting link.
    pub listening: Option<crate::steps::voice_control::listening::ListeningRun>,
}
