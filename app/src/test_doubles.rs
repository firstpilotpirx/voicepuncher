//! The method bodies the test doubles of the ports share.
//!
//! A recorder, an overlay and a spotter that take no part in voice control. Each macro is
//! invoked inside the `impl` of its port, so every double still names what it does.

/// The three live-editing methods of a recorder that records whole buffers only: it cuts
/// nothing, takes no lead-in and keeps no recording. Invoked inside its `impl AudioRecorder`.
#[macro_export]
macro_rules! records_whole_buffers_only {
    () => {
        fn truncate_at(&mut self, _cut: ::std::time::Instant) {}
        fn begin_with(&mut self, _lead_in: $crate::core::domain::AudioBuffer16k) {}
        fn recording(&mut self) -> ::std::option::Option<$crate::core::domain::AudioBuffer16k> {
            ::std::option::Option::None
        }
    };
}

/// The voice control signs of an overlay that shows none and a hidden live transcript.
/// Invoked inside its `impl VisualIndicator`.
#[macro_export]
macro_rules! shows_no_voice_control_signs {
    () => {
        fn show_command_sign(&self, _command: $crate::core::domain::VoiceCommand) {}
        fn show_commands_unavailable(&self) {}
        fn hide_commands_unavailable(&self) {}
        fn live_transcript_view(
            &self,
        ) -> ::std::boxed::Box<
            dyn $crate::core::application::live_transcript_view::LiveTranscriptView,
        > {
            ::std::boxed::Box::new(
                $crate::core::application::live_transcript_view::HiddenLiveTranscript,
            )
        }
    };
}

/// `heard_since` of a spotter that holds no sound: an empty buffer. Invoked inside its
/// `impl CommandSpotter`.
#[macro_export]
macro_rules! holds_no_heard_sound {
    () => {
        fn heard_since(
            &self,
            _since: ::std::time::Instant,
        ) -> $crate::core::domain::AudioBuffer16k {
            $crate::core::domain::AudioBuffer16k::new(::std::vec::Vec::new())
        }
    };
}

/// One second of steady sound, what a stand-in recorder hands over.
pub fn one_second_of_sound() -> crate::core::domain::AudioBuffer16k {
    let samples = crate::core::domain::AudioBuffer16k::SAMPLE_RATE_HZ as usize;
    crate::core::domain::AudioBuffer16k::new(vec![1_000; samples])
}

/// The methods of a recorder that hands over one second of sound at the stop and records no
/// parts, besides its `start`. Invoked inside its `impl AudioRecorder`.
#[macro_export]
macro_rules! hands_over_one_second {
    () => {
        fn stop(&mut self) -> ::anyhow::Result<$crate::core::domain::AudioBuffer16k> {
            ::std::result::Result::Ok($crate::test_doubles::one_second_of_sound())
        }
        fn start_live_chunks(
            &mut self,
            _cfg: $crate::core::application::ChunkingCfg,
        ) -> ::anyhow::Result<
            ::std::option::Option<
                ::std::sync::mpsc::Receiver<$crate::core::application::audio_recorder::LiveChunk>,
            >,
        > {
            ::std::result::Result::Ok(::std::option::Option::None)
        }
        fn flush_live_chunks(&mut self) {}
        $crate::records_whole_buffers_only!();
    };
}

/// The dictation states of an overlay that shows none of them.
///
/// Recording, speech recognition, the wait for a text field, the Pasted sign, hiding and
/// failure notices. Invoked inside its `impl VisualIndicator`.
#[macro_export]
macro_rules! shows_no_dictation_states {
    () => {
        fn show_recording(&self) {}
        fn show_recognizing_speech(&self) {}
        fn show_waiting_for_focus(&self) {}
        fn show_pasted(&self) {}
        fn hide(&self) {}
        fn show_failure_notice(&self, _notice: &$crate::core::domain::FailureNotice) {}
    };
}

/// A scenario's ordered log of what the stand-ins were asked to do.
pub type Log<E> = std::sync::Arc<std::sync::Mutex<Vec<E>>>;

/// A paster that notes every text it pastes or leaves in the clipboard as `pasted` of it.
pub struct NotingPaster<E> {
    pub log: Log<E>,
    pub pasted: fn(String) -> E,
}

impl<E> NotingPaster<E> {
    pub fn new(log: &Log<E>, pasted: fn(String) -> E) -> Self {
        Self { log: std::sync::Arc::clone(log), pasted }
    }
}

impl<E: Send> crate::core::application::TextPaster for NotingPaster<E> {
    fn paste(&mut self, text: &crate::core::domain::CleanedText) -> anyhow::Result<()> {
        let event = (self.pasted)(text.as_str().to_owned());
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
        Ok(())
    }

    fn leave_in_clipboard(
        &mut self,
        text: &crate::core::domain::CleanedText,
    ) -> anyhow::Result<()> {
        self.paste(text)
    }
}

/// A recorder that notes each start as `started` and hands over one second of sound.
pub struct OneSecondRecorder<E> {
    pub log: Log<E>,
    pub started: E,
}

impl<E: Clone> OneSecondRecorder<E> {
    pub fn new(log: &Log<E>, started: E) -> Self {
        Self { log: std::sync::Arc::clone(log), started }
    }
}

impl<E: Clone + Send> crate::core::application::AudioRecorder for OneSecondRecorder<E> {
    fn start(&mut self) -> anyhow::Result<()> {
        let event = self.started.clone();
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
        Ok(())
    }

    crate::hands_over_one_second!();
}

/// The start and the stop of a recorder stand-in that records nothing.
#[macro_export]
macro_rules! records_nothing {
    () => {
        fn start(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        fn stop(&mut self) -> anyhow::Result<$crate::core::domain::AudioBuffer16k> {
            Ok($crate::core::domain::AudioBuffer16k::new(Vec::new()))
        }
    };
}

/// The `AudioRecorder` methods of a recorder that cuts no live chunks.
#[macro_export]
macro_rules! no_live_chunks {
    () => {
        fn start_live_chunks(
            &mut self,
            _cfg: $crate::core::application::ChunkingCfg,
        ) -> anyhow::Result<
            Option<std::sync::mpsc::Receiver<$crate::core::application::audio_recorder::LiveChunk>>,
        > {
            Ok(None)
        }

        fn flush_live_chunks(&mut self) {}
    };
}

/// A paster that pastes nothing and leaves nothing in the clipboard.
pub struct DiscardingPaster;

impl crate::core::application::TextPaster for DiscardingPaster {
    fn paste(&mut self, _text: &crate::core::domain::CleanedText) -> anyhow::Result<()> {
        Ok(())
    }

    fn leave_in_clipboard(
        &mut self,
        _text: &crate::core::domain::CleanedText,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

/// One second of sound whose first sample is `n`, numbering it among others.
pub fn numbered_second(n: usize) -> crate::core::domain::AudioBuffer16k {
    let mut samples = one_second_of_sound().samples().to_vec();
    samples[0] = i16::try_from(n).unwrap_or(i16::MAX);
    crate::core::domain::AudioBuffer16k::new(samples)
}

/// What the command spotting service reports when the user says `phrase` alone, the speech
/// starting at `segment_start`: the phrase 100 ms in, 800 ms long, and 100 ms of silence.
pub fn said_alone(
    phrase: &str,
    segment_start: std::time::Instant,
) -> [crate::core::domain::SpottingEvent; 3] {
    use crate::core::domain::SpottingEvent;
    let start = segment_start + std::time::Duration::from_millis(100);
    let end = start + std::time::Duration::from_millis(800);
    [
        SpottingEvent::SpeechStarted(segment_start),
        SpottingEvent::PhraseSpotted { phrase: phrase.to_owned(), start, end, timing: None },
        SpottingEvent::SpeechEnded(end + std::time::Duration::from_millis(100)),
    ]
}

/// The requests a stand-in model host took: each file and the byte it asked to start from.
pub type ModelRequests = std::sync::Arc<std::sync::Mutex<Vec<(String, u64)>>>;

/// Notes a request for `url` that resumes the part `sink` holds; the byte it starts from.
pub fn note_request(
    requests: &ModelRequests,
    url: &str,
    sink: &dyn crate::core::application::PartSink,
) -> u64 {
    let from = sink.len();
    requests
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push((url.to_owned(), from));
    from
}

/// A test run whose pipeline starts at its first input.
pub trait StartedOnDemand {
    /// Where the running pipeline's inputs go, none before it started.
    fn inputs(
        &mut self,
    ) -> &mut Option<std::sync::mpsc::Sender<crate::core::application::PipelineInput>>;

    /// Starts the pipeline; its inputs.
    fn start_pipeline(
        &mut self,
    ) -> std::sync::mpsc::Sender<crate::core::application::PipelineInput>;

    /// Starts the pipeline unless it already runs.
    fn ensure_started(&mut self) {
        if self.inputs().is_none() {
            let inputs = self.start_pipeline();
            *self.inputs() = Some(inputs);
        }
    }

    /// Sends `input` to the pipeline, started first when it does not run yet.
    fn send(&mut self, input: crate::core::application::PipelineInput) -> anyhow::Result<()> {
        use anyhow::Context;
        self.ensure_started();
        let inputs = self.inputs().as_ref().context("pipeline not running")?;
        inputs.send(input)?;
        Ok(())
    }
}

/// A sound player that plays nothing and an overlay that shows nothing.
pub struct Silent;

impl crate::core::application::VisualIndicator for Silent {
    crate::shows_no_dictation_states!();
    crate::shows_no_voice_control_signs!();
}

impl crate::core::application::SoundPlayer for Silent {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {}
    fn play_listening_off(&self) {}
}

/// The parts a recorder stand-in sends while the user speaks, one second of sound each: one
/// when live chunking starts and the last one when it is flushed.
#[derive(Default)]
pub struct OneSecondParts(
    Option<std::sync::mpsc::Sender<crate::core::application::audio_recorder::LiveChunk>>,
);

impl OneSecondParts {
    /// Starts sending parts: the first one is sent at once.
    pub fn start(
        &mut self,
    ) -> std::sync::mpsc::Receiver<crate::core::application::audio_recorder::LiveChunk> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.0 = Some(tx);
        self.send();
        rx
    }

    /// Sends the last part; the transcription of the parts ends with it.
    pub fn flush(&mut self) {
        self.send();
        self.0 = None;
    }

    fn send(&self) {
        if let Some(parts) = &self.0 {
            let part = crate::core::application::audio_recorder::LiveChunk {
                audio: one_second_of_sound(),
                enqueued_at: std::time::Instant::now(),
            };
            let _ = parts.send(part);
        }
    }
}

/// The names the test doubles and the steps that drive the pipeline use most, for one glob
/// import in place of the same long list in every file of them.
pub mod prelude {
    pub use std::collections::HashMap;
    pub use std::path::{Path, PathBuf};
    pub use std::process::{Child, Command, Output, Stdio};
    pub use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    pub use std::sync::mpsc::{self, Receiver, Sender};
    pub use std::sync::{Arc, Mutex};
    pub use std::thread::JoinHandle;
    pub use std::time::{Duration, Instant};

    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use super::StartedOnDemand;

    pub use crate::core::application::audio_recorder::LiveChunk;
    pub use crate::core::application::pipeline::{Pipeline, PipelineConfig, PipelineParts};
    pub use crate::core::application::{
        AudioRecorder, ChunkingCfg, CommandSpotter, FocusProbe, FocusWaitCfg, HotkeyListener,
        KeyPresser, PipelineInput, SoundPlayer, TextPaster, Transcriber, VisualIndicator,
    };
    pub use crate::core::domain::{
        AudioBuffer16k, CleanedText, FailingPart, FailureNotice, FocusedElement, PttSignal,
        RawTranscript, RecordingMode, SpottingEvent, VoiceCommand,
    };
}
