mod archiving;
mod dictation;
mod failure_notices;
mod live_transcript;
mod live_worker;
mod parts;
mod processing;
mod recording;
mod recording_failure;
mod voice_control;

pub use parts::{PipelineConfig, PipelineParts};

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::core::application::pipeline::failure_notices::past_recording_not_started;
use crate::core::application::pipeline::live_transcript::RecordingEnd;
use crate::core::application::{
    hotkey_pressed_during_wait, AudioRecorder, DictationArchive, DictationStart, FocusProbe,
    FocusWaitCfg, NoActivitySink, PipelineActivity, PipelineActivitySink, PipelineInput,
    SoundPlayer, TextPaster, Transcriber, VisualIndicator,
};
use crate::core::domain::{FailureNotice, Permission, PttSignal, RecordingMode};

use self::dictation::Dictation;
use self::voice_control::Voice;

/// The cue a recording starts with, played before the recorder opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartCue {
    Start,
    Restart,
}

pub(crate) struct LiveTranscriptionSession {
    pub(super) results_rx: mpsc::Receiver<LiveSessionResult>,
    pub(super) recording_end: RecordingEnd,
}

pub(super) struct LiveSessionResult {
    pub(super) texts: Vec<String>,
    /// Chunks speech recognition failed on.
    pub(super) lost_chunks: usize,
}

/// The one transcriber the pipeline and every live worker use: a worker that dies leaves it
/// here, and one still on a hung request holds it only until that request ends.
pub(super) type SharedTranscriber = Arc<Mutex<Box<dyn Transcriber>>>;

/// The shared transcriber; one a dead worker's panic left behind is taken over.
pub(super) fn lock_transcriber(
    transcriber: &SharedTranscriber,
) -> MutexGuard<'_, Box<dyn Transcriber>> {
    transcriber
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub struct Pipeline {
    recorder: Box<dyn AudioRecorder>,
    transcriber: SharedTranscriber,
    paster: Box<dyn TextPaster>,
    sound_player: Box<dyn SoundPlayer>,
    visual_indicator: Box<dyn VisualIndicator>,
    cfg: PipelineConfig,
    /// The running dictation, from its start cue until it is stopped or abandoned.
    dictation: Option<Dictation>,
    /// Set when a dictation's text waits for a text field to have focus before it is pasted.
    focus: Option<FocusWait>,
    inbox: Inbox,
    voice: Voice,
    sinks: Sinks,
}

/// How a dictation's text waits for text input focus before it is pasted (see `insert_text`).
struct FocusWait {
    probe: Box<dyn FocusProbe>,
    cfg: FocusWaitCfg,
}

/// The inputs: the channel the runtime sends them on, and the ones read ahead during a
/// focus wait, handled before the channel is read again.
#[derive(Default)]
struct Inbox {
    receiver: Option<mpsc::Receiver<PipelineInput>>,
    pending: VecDeque<PipelineInput>,
}

impl Inbox {
    fn next(&mut self) -> anyhow::Result<PipelineInput> {
        if let Some(input) = self.pending.pop_front() {
            return Ok(input);
        }
        self.receiver
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("pipeline not running"))?
            .recv()
            .map_err(|_| anyhow::anyhow!("pipeline input channel closed"))
    }

    /// Whether the hotkey was pressed meanwhile; the other inputs read wait their turn.
    fn hotkey_pressed(&mut self) -> bool {
        self.receiver
            .as_ref()
            .is_some_and(|rx| hotkey_pressed_during_wait(rx, &mut self.pending))
    }
}

/// Where the pipeline reports: each closed dictation to the archive, its activity (idle,
/// recording, processing) after every input.
struct Sinks {
    archive: Option<Box<dyn DictationArchive>>,
    activity: Box<dyn PipelineActivitySink>,
}

impl Pipeline {
    pub fn new(parts: PipelineParts, cfg: PipelineConfig) -> Self {
        Self {
            recorder: parts.recorder,
            transcriber: Arc::new(Mutex::new(parts.transcriber)),
            paster: parts.paster,
            sound_player: parts.sound_player,
            visual_indicator: parts.visual_indicator,
            cfg,
            dictation: None,
            focus: None,
            inbox: Inbox::default(),
            voice: Voice::Off,
            sinks: Sinks { archive: None, activity: Box::new(NoActivitySink) },
        }
    }

    /// A stopped dictation waits at most `wait` for its transcript.
    pub fn with_stop_wait(mut self, wait: Duration) -> Self {
        self.cfg.stop_wait = wait;
        self
    }

    /// Publishes the activity to `sink` after every input, so a restart can wait for Idle.
    pub fn with_activity_sink(mut self, sink: Box<dyn PipelineActivitySink>) -> Self {
        self.sinks.activity = sink;
        self
    }

    /// Waits for text input focus before a dictation is pasted (see `insert_text`).
    pub fn with_focus_wait(mut self, probe: Box<dyn FocusProbe>, cfg: FocusWaitCfg) -> Self {
        self.focus = Some(FocusWait { probe, cfg });
        self
    }

    pub fn run(&mut self, rx: mpsc::Receiver<PipelineInput>) -> anyhow::Result<()> {
        // kept in `self` so a focus wait inside `step` sees a hotkey press
        self.inbox.receiver = Some(rx);
        self.turn_listening_on();
        loop {
            match self.inbox.next()? {
                PipelineInput::Shutdown => return self.shut_down(),
                PipelineInput::PermissionRevoked(permission) => {
                    return self.shut_down_revoked(permission)
                }
                other => self.handle(other)?,
            }
        }
    }

    /// Handles one input and publishes the activity it leaves the pipeline in; a dictation
    /// being stopped counts as processing until its paste.
    fn handle(&mut self, input: PipelineInput) -> anyhow::Result<()> {
        if self.is_recording() {
            self.sinks.activity.publish(PipelineActivity::Processing);
        }
        let outcome = self.dispatch(input);
        self.publish_activity();
        outcome
    }

    fn dispatch(&mut self, input: PipelineInput) -> anyhow::Result<()> {
        match input {
            PipelineInput::Hotkey(signal) => self.step(signal),
            PipelineInput::Shutdown => self.shut_down(),
            PipelineInput::PermissionRevoked(permission) => self.shut_down_revoked(permission),
            PipelineInput::RecordingFailed(reason) => self.recording_failed(&reason),
            spotting => match self.voice {
                Voice::On(_) => self.handle_with_voice_control(spotting),
                Voice::Off => {
                    log::warn!("ignoring {spotting:?}: voice control is off");
                    Ok(())
                }
            },
        }
    }

    fn publish_activity(&self) {
        self.sinks.activity.publish(if self.is_recording() {
            PipelineActivity::Recording
        } else {
            PipelineActivity::Idle
        });
    }

    /// Ends this pipeline: a recording in progress is cancelled, listening stops.
    fn shut_down(&mut self) -> anyhow::Result<()> {
        log::info!("pipeline shutting down");
        if self.is_recording() {
            self.cancel_recording()?;
        }
        self.voice.turn_listening_off();
        self.publish_activity();
        Ok(())
    }

    /// Ends this pipeline as a shutdown does; a recording in progress ends with a notice
    /// naming the revoked permission.
    fn shut_down_revoked(&mut self, permission: Permission) -> anyhow::Result<()> {
        let was_recording = self.is_recording();
        self.shut_down()?;
        if was_recording {
            self.visual_indicator
                .show_failure_notice(&FailureNotice::PermissionRevoked(permission));
        }
        Ok(())
    }

    fn step(&mut self, signal: PttSignal) -> anyhow::Result<()> {
        let recording = self.is_recording();
        let state = if recording { "Recording" } else { "Idle" };
        log::info!("pipeline recv {signal:?} in state {state}");
        match (recording, signal, self.cfg.recording_mode) {
            (false, PttSignal::Pressed, _) => past_recording_not_started(self.start_recording()),
            (true, PttSignal::Released, RecordingMode::Hold)
            | (true, PttSignal::Pressed, RecordingMode::Toggle) => self.stop_recording(),
            (true, PttSignal::Cancel, _) => self.cancel_recording(),
            (_, sig, _) => {
                log::warn!("ignoring {sig:?} in state {state}");
                Ok(())
            }
        }
    }

    fn start_recording(&mut self) -> anyhow::Result<()> {
        self.start_recording_from(None, StartCue::Start)
    }

    /// A dictation that starts now, traced when there is an archive to keep it.
    fn new_dictation(&self, start: DictationStart) -> Dictation {
        Dictation::new(start, self.sinks.archive.is_some())
    }

    fn is_recording(&self) -> bool {
        self.dictation.is_some()
    }
}

#[cfg(test)]
pub(crate) mod tests;
