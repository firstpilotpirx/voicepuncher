//! The stand-ins of the hands free scenarios.

#![allow(unused_imports)]

use super::{
    mpsc, AudioBuffer16k, AudioRecorder, AudioStream, ChunkingCfg, CleanedText, CommandSpotter,
    Context, Event, Events, FailureNotice, Instant, KeyPresser, LiveChunk, PipelineInput,
    RawTranscript, Receiver, Result, Sender, SoundPlayer, TextPaster, Transcriber, VisualIndicator,
    SAY_SAMPLE_RATE_HZ, SENTENCE,
};
use voicepuncher::core::application::live_transcript_view::{
    HiddenLiveTranscript, LiveTranscriptView,
};
use voicepuncher::core::domain::VoiceCommand;

/// The command spotting adapter's streaming path, with `say` speech in place of the microphone.
pub(super) struct SpeakerSpotter {
    pub(super) url: String,
    pub(super) inputs: Sender<PipelineInput>,
    pub(super) heard: Option<Receiver<Vec<f32>>>,
    pub(super) stream: Option<AudioStream>,
    pub(super) events: Events,
}

impl CommandSpotter for SpeakerSpotter {
    fn probe(&self) -> Result<()> {
        Ok(())
    }

    fn start_listening(&mut self, commands: &[VoiceCommand]) -> Result<()> {
        let heard = self.heard.take().context("the speaker is already in use")?;
        let stream = AudioStream::start(
            &self.url,
            commands,
            Instant::now(),
            SAY_SAMPLE_RATE_HZ,
            heard,
            self.inputs.clone(),
        )?;
        self.stream = Some(stream);
        self.events.lock().unwrap().push(Event::Listening);
        Ok(())
    }

    fn listen_for(&mut self, commands: &[VoiceCommand]) {
        if let Some(stream) = self.stream.as_ref() {
            stream.listen_for(commands);
        }
    }

    fn stop_listening(&mut self) {
        if let Some(stream) = self.stream.take() {
            stream.stop();
        }
    }
    voicepuncher::holds_no_heard_sound!();
}

pub(super) struct StandInRecorder(pub(super) Events);

impl AudioRecorder for StandInRecorder {
    fn start(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::RecordingStarted);
        Ok(())
    }

    fn stop(&mut self) -> Result<AudioBuffer16k> {
        let one_second = AudioBuffer16k::SAMPLE_RATE_HZ as usize;
        Ok(AudioBuffer16k::new(vec![1_000i16; one_second]))
    }

    fn start_live_chunks(&mut self, _cfg: ChunkingCfg) -> Result<Option<Receiver<LiveChunk>>> {
        Ok(None)
    }

    fn flush_live_chunks(&mut self) {}
    voicepuncher::records_whole_buffers_only!();
}

pub(super) struct FixedTranscript;

impl Transcriber for FixedTranscript {
    fn transcribe(&mut self, _audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        Ok(Some(RawTranscript::new(SENTENCE.to_owned())?))
    }
}

pub(super) struct RecordingPaster(pub(super) Events);

impl TextPaster for RecordingPaster {
    fn paste(&mut self, text: &CleanedText) -> Result<()> {
        let pasted = Event::Pasted(text.as_str().to_owned());
        self.0.lock().unwrap().push(pasted);
        Ok(())
    }

    fn leave_in_clipboard(&mut self, text: &CleanedText) -> Result<()> {
        self.paste(text)
    }
}

pub(super) struct RecordingKeys(pub(super) Events);

impl KeyPresser for RecordingKeys {
    fn press_enter(&mut self) -> Result<()> {
        self.0.lock().unwrap().push(Event::ReturnPressed);
        Ok(())
    }

    fn clear_all_text(&mut self) -> Result<()> {
        Ok(())
    }

    fn paste(&mut self) -> Result<()> {
        Ok(())
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
    fn show_recording(&self) {}
    fn show_recognizing_speech(&self) {}
    fn show_refining_text(&self) {}
    fn show_waiting_for_focus(&self) {}
    fn show_pasted(&self) {}
    fn hide(&self) {}
    fn show_failure_notice(&self, _notice: &FailureNotice) {}
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
