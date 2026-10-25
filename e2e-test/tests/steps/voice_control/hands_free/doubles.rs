//! The stand-ins of the hands free scenarios.

#![allow(unused_imports)]

use super::{
    mpsc, AudioBuffer16k, AudioRecorder, AudioStream, ChunkingCfg, CleanedText, CommandSpotter,
    Context, Event, Events, FailureNotice, Instant, KeyPresser, LiveChunk, MicrophoneAudio,
    PipelineInput, RawTranscript, Receiver, Result, Sender, SoundPlayer, TextPaster, Transcriber,
    VisualIndicator, SAY_SAMPLE_RATE_HZ, SENTENCE,
};
use voicepuncher::core::application::live_transcript_view::{
    HiddenLiveTranscript, LiveTranscriptView,
};
use voicepuncher::test_doubles::prelude::*;

pub(super) type StandInRecorder = voicepuncher::test_doubles::OneSecondRecorder<Event>;

pub(super) struct FixedTranscript;

impl Transcriber for FixedTranscript {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(Some(RawTranscript::new(SENTENCE.to_owned())?))
    }
}

/// Plays nothing; notes the listening-on and listening-off sounds.
pub(super) struct ListeningSounds(pub(super) Events);

impl SoundPlayer for ListeningSounds {
    fn play_start(&self) {}
    fn play_stop(&self) {}
    fn play_cancel(&self) {}
    fn play_listening_on(&self) {
        self.0.lock().unwrap().push(Event::ListeningOnSound);
    }
    fn play_listening_off(&self) {
        self.0.lock().unwrap().push(Event::ListeningOffSound);
    }
}

/// Passes every input on to `pipeline`, noting the losses and returns of the command
/// spotting service on the way.
pub(super) fn tee_inputs(pipeline: Sender<PipelineInput>, events: Events) -> Sender<PipelineInput> {
    let (tee, inputs) = mpsc::channel();
    std::thread::spawn(move || {
        for input in inputs {
            let noted = match input {
                PipelineInput::SpottingLost => Some(Event::SpottingLost),
                PipelineInput::SpottingBack => Some(Event::SpottingBack),
                _ => None,
            };
            if let Some(event) = noted {
                events.lock().unwrap().push(event);
            }
            if pipeline.send(input).is_err() {
                return;
            }
        }
    });
    tee
}

/// Shows nothing; notes when the sign that voice commands do not work comes and goes.
pub(super) struct SignOverlay(pub(super) Events);

impl VisualIndicator for SignOverlay {
    voicepuncher::shows_no_dictation_states!();
    fn show_commands_unavailable(&self) {
        self.0.lock().unwrap().push(Event::CommandsUnavailableShown);
    }
    fn hide_commands_unavailable(&self) {
        self.0
            .lock()
            .unwrap()
            .push(Event::CommandsUnavailableHidden);
    }
    fn show_command_sign(&self, _command: VoiceCommand) {}
    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        Box::new(HiddenLiveTranscript)
    }
}
