pub mod audio_level_sink;
pub mod audio_level_source;
pub mod audio_recorder;
pub mod chunking;
pub mod command_spotter;
pub mod dictation_archive;
pub mod focus_probe;
pub mod hotkey_listener;
pub mod input_devices;
pub mod key_presser;
pub mod live_transcript_view;
pub mod local_services;
pub mod login_item;
pub mod model_download;
pub mod permission_probe;
pub mod permissions_gate;
pub mod pipeline;
pub mod pipeline_activity;
pub mod pipeline_input;
pub mod sound_player;
pub mod text_insertion;
pub mod text_paster;
pub mod transcriber;
pub mod transcript_filter;
pub mod visual_indicator;

pub use audio_level_sink::AudioLevelSink;
pub use audio_level_source::AudioLevelSource;
pub use audio_recorder::AudioRecorder;
pub use chunking::ChunkingCfg;
pub use command_spotter::CommandSpotter;
pub use dictation_archive::{
    ArchiveEntry, DictationArchive, DictationOutcome, DictationStart, StageTiming,
};
pub use focus_probe::FocusProbe;
pub use hotkey_listener::HotkeyListener;
pub use input_devices::InputDevices;
pub use key_presser::KeyPresser;
pub use live_transcript_view::LiveTranscriptView;
pub use local_services::{
    Clock, ProbeAnswer, ServiceLauncher, ServiceProbe, ServiceProcess, ServiceRuntime, Services,
    TICK,
};
pub use login_item::{LoginItem, LoginItemStatus};
pub use model_download::{
    DiskSpace, DownloadEvent, DownloadFailure, DownloadObserver, DownloadSummary, FetchError,
    ModelDownload, ModelFetcher, ModelRow, ModelStore, ModelsReport, PartSink,
};
pub use permission_probe::PermissionProbe;
pub use permissions_gate::{PermissionReport, PermissionsGate};
pub use pipeline_activity::{NoActivitySink, PipelineActivity, PipelineActivitySink};
pub use pipeline_input::PipelineInput;
pub use sound_player::SoundPlayer;
pub use text_insertion::{
    hotkey_pressed_during_wait, insert_text, FocusWait, FocusWaitCfg, Insertion, WaitEnd,
};
pub use text_paster::TextPaster;
pub use transcriber::Transcriber;
pub use visual_indicator::{OnDrawn, VisualIndicator};

/// The mocks of the ports, for one glob import in the unit tests.
#[cfg(test)]
pub(crate) mod prelude {
    pub(crate) use super::audio_recorder::MockAudioRecorder;
    pub(crate) use super::command_spotter::MockCommandSpotter;
    pub(crate) use super::hotkey_listener::MockHotkeyListener;
    pub(crate) use super::key_presser::MockKeyPresser;
    pub(crate) use super::sound_player::MockSoundPlayer;
    pub(crate) use super::text_paster::MockTextPaster;
    pub(crate) use super::transcriber::MockTranscriber;
    pub(crate) use super::visual_indicator::MockVisualIndicator;

    /// An overlay that takes every dictation state and sign it is shown.
    pub(crate) fn accepting_overlay() -> MockVisualIndicator {
        let mut overlay = MockVisualIndicator::new();
        overlay.expect_show_recording().return_const(());
        overlay
            .expect_show_recording_then()
            .returning(|on_drawn| on_drawn(std::time::Instant::now()));
        overlay.expect_show_recognizing_speech().return_const(());
        overlay.expect_show_waiting_for_focus().return_const(());
        overlay.expect_show_pasted().return_const(());
        overlay.expect_hide().return_const(());
        overlay.expect_show_command_sign().return_const(());
        overlay.expect_show_failure_notice().return_const(());
        overlay
    }
}
