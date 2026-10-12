use crate::core::application::{CommandSpotter, KeyPresser, PipelineInput};
use crate::core::domain::{
    PttSignal, SpokenCommand, SpottingEvent, VoiceCommand, VoiceCommandRecognizer,
};

use super::Pipeline;

pub(super) struct VoiceControl {
    spotter: Box<dyn CommandSpotter>,
    key_presser: Box<dyn KeyPresser>,
    recognizer: VoiceCommandRecognizer,
    listening: bool,
}

impl Pipeline {
    /// Turns voice control on: the hotkey controls listening and spoken commands drive
    /// dictations and key presses.
    pub fn with_voice_control(
        mut self,
        spotter: Box<dyn CommandSpotter>,
        key_presser: Box<dyn KeyPresser>,
    ) -> Self {
        self.voice = Some(VoiceControl {
            spotter,
            key_presser,
            recognizer: VoiceCommandRecognizer::new(),
            listening: false,
        });
        self
    }

    pub(super) fn handle_with_voice_control(&mut self, input: PipelineInput) -> anyhow::Result<()> {
        match &input {
            PipelineInput::Spotting(event) => log::debug!("voice control recv {event:?}"),
            other => log::info!(
                "voice control recv {other:?}, listening={}",
                self.listening()
            ),
        }
        match input {
            PipelineInput::Hotkey(PttSignal::Pressed) => self.toggle_listening(),
            PipelineInput::Hotkey(PttSignal::Cancel) if self.is_recording() => {
                self.cancel_dictation()
            }
            PipelineInput::Hotkey(_) => Ok(()),
            PipelineInput::Spotting(event) => {
                if let Some(spoken) = self.recognize(event) {
                    self.carry_out_or_log(spoken);
                }
                Ok(())
            }
            PipelineInput::SpottingLost => self.lose_spotting(),
        }
    }

    fn voice(&mut self) -> &mut VoiceControl {
        self.voice.as_mut().expect("voice control is on")
    }

    fn listening(&self) -> bool {
        self.voice.as_ref().is_some_and(|voice| voice.listening)
    }

    fn toggle_listening(&mut self) -> anyhow::Result<()> {
        if !self.listening() {
            self.turn_listening_on();
            return Ok(());
        }
        if self.is_recording() {
            self.stop_recording()?;
        }
        self.turn_listening_off();
        Ok(())
    }

    pub(super) fn turn_listening_on(&mut self) {
        log::info!("voice control: turning listening on…");
        let voice = self.voice();
        voice.recognizer = VoiceCommandRecognizer::new();
        if let Err(e) = voice.spotter.start_listening(&VoiceCommand::STANDBY) {
            log::error!("voice control: cannot listen, command spotting service: {e:#}");
            return;
        }
        voice.listening = true;
        self.sound_player.play_listening_on();
        log::info!("voice control: listening");
    }

    fn turn_listening_off(&mut self) {
        let voice = self.voice();
        voice.spotter.stop_listening();
        voice.listening = false;
        self.sound_player.play_listening_off();
        log::info!("voice control: not listening");
    }

    fn recognize(&mut self, event: SpottingEvent) -> Option<SpokenCommand> {
        if !self.listening() {
            return None;
        }
        self.voice().recognizer.feed(event)
    }

    /// A command that fails (the microphone is busy, a key press is refused) is logged;
    /// the app keeps listening for the next one.
    fn carry_out_or_log(&mut self, spoken: SpokenCommand) {
        if let Err(e) = self.carry_out(spoken) {
            log::error!(
                "voice control: \"{}\" failed: {e:#}",
                spoken.command.phrase()
            );
            self.visual_indicator.hide();
        }
    }

    fn carry_out(&mut self, spoken: SpokenCommand) -> anyhow::Result<()> {
        let phrase = spoken.command.phrase();
        let accepted: &[VoiceCommand] = if self.is_recording() {
            &VoiceCommand::DICTATING
        } else {
            &VoiceCommand::STANDBY
        };
        if !accepted.contains(&spoken.command) {
            log::info!("voice control: \"{phrase}\" ignored in this state");
            return Ok(());
        }
        log::info!("voice control: \"{phrase}\"");
        match spoken.command {
            VoiceCommand::StartDictation => {
                self.start_recording()?;
                self.voice().spotter.listen_for(&VoiceCommand::DICTATING);
                Ok(())
            }
            VoiceCommand::StopDictation => {
                self.voice().spotter.listen_for(&VoiceCommand::STANDBY);
                self.recorder.truncate_at(spoken.started_at);
                self.stop_recording()
            }
            VoiceCommand::CancelDictation => self.cancel_dictation(),
            VoiceCommand::PressEnter => self.press_keys(|keys| keys.press_enter()),
            VoiceCommand::ClearAllText => self.press_keys(|keys| keys.clear_all_text()),
        }
    }

    fn cancel_dictation(&mut self) -> anyhow::Result<()> {
        self.voice().spotter.listen_for(&VoiceCommand::STANDBY);
        self.cancel_recording()
    }

    fn press_keys(
        &mut self,
        press: impl FnOnce(&mut dyn KeyPresser) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        match press(self.voice().key_presser.as_mut()) {
            Ok(()) => self.sound_player.play_command(),
            Err(e) => log::error!("voice control: key press failed: {e:#}"),
        }
        Ok(())
    }

    fn lose_spotting(&mut self) -> anyhow::Result<()> {
        if !self.listening() {
            return Ok(());
        }
        log::error!(
            "voice control: the command spotting service stopped answering; listening is off"
        );
        if self.is_recording() {
            self.stop_recording()?;
        }
        self.turn_listening_off();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::command_spotter::MockCommandSpotter;
    use crate::core::application::key_presser::MockKeyPresser;
    use crate::core::application::pipeline::PipelineConfig;
    use crate::core::application::sound_player::MockSoundPlayer;
    use crate::core::application::text_paster::MockTextPaster;
    use crate::core::application::text_refiner::MockTextRefiner;
    use crate::core::application::transcriber::MockTranscriber;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::application::{ChunkingCfg, PipelineInput};
    use crate::core::domain::{
        AudioBuffer16k, CleanedText, PttSignal, RawTranscript, RecordingMode, SpottingEvent,
        VoiceCommand,
    };

    const PHRASE_LEAD: Duration = Duration::from_millis(100);
    const PHRASE_LENGTH: Duration = Duration::from_millis(800);

    fn sounds() -> MockSoundPlayer {
        let mut sounds = MockSoundPlayer::new();
        sounds.expect_play_start().return_const(());
        sounds.expect_play_stop().return_const(());
        sounds.expect_play_cancel().return_const(());
        sounds.expect_play_listening_on().return_const(());
        sounds.expect_play_listening_off().return_const(());
        sounds.expect_play_command().return_const(());
        sounds
    }

    fn overlay() -> MockVisualIndicator {
        let mut overlay = MockVisualIndicator::new();
        overlay.expect_show_recording().return_const(());
        overlay.expect_show_processing().return_const(());
        overlay.expect_hide().return_const(());
        overlay
    }

    fn idle_recorder() -> MockAudioRecorder {
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().never();
        recorder
    }

    fn recording_recorder() -> MockAudioRecorder {
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().returning(|| Ok(()));
        recorder.expect_start_live_chunks().returning(|_| Ok(None));
        recorder.expect_flush_live_chunks().return_const(());
        recorder
            .expect_stop()
            .returning(|| Ok(AudioBuffer16k::new(vec![0i16; 16_000])));
        recorder
    }

    fn answering_transcriber() -> MockTranscriber {
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .returning(|_| Ok(Some(RawTranscript::new("hello".into()).unwrap())));
        transcriber
    }

    fn passthrough_refiner() -> MockTextRefiner {
        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .returning(|raw| Ok(CleanedText::new(raw.as_str().to_owned()).unwrap()));
        refiner
    }

    fn lenient_spotter() -> MockCommandSpotter {
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        spotter.expect_listen_for().return_const(());
        spotter.expect_stop_listening().return_const(());
        spotter
    }

    fn pipeline(
        recorder: MockAudioRecorder,
        paster: MockTextPaster,
        spotter: MockCommandSpotter,
        mode: RecordingMode,
    ) -> Pipeline {
        let cfg = PipelineConfig {
            min_press_ms: 0,
            recording_mode: mode,
            chunking: ChunkingCfg {
                chunk_threshold_secs: 0,
                ..ChunkingCfg::default()
            },
        };
        Pipeline::new(
            Box::new(recorder),
            Box::new(answering_transcriber()),
            Box::new(passthrough_refiner()),
            Box::new(paster),
            Box::new(sounds()),
            Box::new(overlay()),
            cfg,
        )
        .with_voice_control(Box::new(spotter), Box::new(MockKeyPresser::new()))
    }

    fn press(pipeline: &mut Pipeline) {
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
    }

    /// Says `phrase` on its own and returns the moment the phrase started.
    fn say(pipeline: &mut Pipeline, phrase: &str) -> Instant {
        let segment_start = Instant::now();
        let start = segment_start + PHRASE_LEAD;
        let end = start + PHRASE_LENGTH;
        let events = [
            SpottingEvent::SpeechStarted(segment_start),
            SpottingEvent::PhraseSpotted {
                phrase: phrase.to_owned(),
                start,
                end,
            },
            SpottingEvent::SpeechEnded(end + PHRASE_LEAD),
        ];
        for event in events {
            pipeline.handle(PipelineInput::Spotting(event)).unwrap();
        }
        start
    }

    #[test]
    fn should_toggle_listening_with_the_hotkey_without_recording() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        spotter
            .expect_start_listening()
            .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
            .once()
            .returning(|_| Ok(()));
        spotter.expect_stop_listening().once().return_const(());
        let mut pipeline = pipeline(
            idle_recorder(),
            MockTextPaster::new(),
            spotter,
            RecordingMode::Toggle,
        );
        // When
        press(&mut pipeline);
        press(&mut pipeline);
        // Then the mock expectations hold
    }

    #[test]
    fn should_keep_listening_on_a_hotkey_release_in_hold_mode() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        spotter
            .expect_start_listening()
            .once()
            .returning(|_| Ok(()));
        spotter.expect_stop_listening().never();
        let mut pipeline = pipeline(
            idle_recorder(),
            MockTextPaster::new(),
            spotter,
            RecordingMode::Hold,
        );
        press(&mut pipeline);
        // When
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Released))
            .unwrap();
        // Then the mock expectations hold
    }

    #[test]
    fn should_cut_the_recording_at_the_stop_phrase_before_stopping_it() {
        // Given
        let mut recorder = recording_recorder();
        let cut_at = std::sync::Arc::new(std::sync::Mutex::new(None));
        let seen = std::sync::Arc::clone(&cut_at);
        recorder
            .expect_truncate_at()
            .once()
            .returning(move |cut| *seen.lock().unwrap() = Some(cut));
        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));
        let mut pipeline = pipeline(recorder, paster, lenient_spotter(), RecordingMode::Toggle);
        press(&mut pipeline);
        say(&mut pipeline, "start dictation");
        // When
        let phrase_start = say(&mut pipeline, "stop dictation");
        // Then
        assert_eq!(*cut_at.lock().unwrap(), Some(phrase_start));
    }

    #[test]
    fn should_listen_for_the_phrases_of_each_state() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        let mut order = mockall::Sequence::new();
        spotter
            .expect_start_listening()
            .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
            .once()
            .in_sequence(&mut order)
            .returning(|_| Ok(()));
        spotter
            .expect_listen_for()
            .with(mockall::predicate::eq(VoiceCommand::DICTATING.to_vec()))
            .once()
            .in_sequence(&mut order)
            .return_const(());
        spotter
            .expect_listen_for()
            .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
            .once()
            .in_sequence(&mut order)
            .return_const(());
        let mut recorder = recording_recorder();
        recorder.expect_truncate_at().return_const(());
        let mut paster = MockTextPaster::new();
        paster.expect_paste().returning(|_| Ok(()));
        let mut pipeline = pipeline(recorder, paster, spotter, RecordingMode::Toggle);
        press(&mut pipeline);
        // When
        say(&mut pipeline, "start dictation");
        say(&mut pipeline, "stop dictation");
        // Then the mock expectations hold
    }

    #[test]
    fn should_paste_and_stop_listening_when_the_service_goes_away_during_a_dictation() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        spotter.expect_listen_for().return_const(());
        spotter.expect_stop_listening().once().return_const(());
        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));
        let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
        press(&mut pipeline);
        say(&mut pipeline, "start dictation");
        // When
        pipeline.handle(PipelineInput::SpottingLost).unwrap();
        // Then the mock expectations hold
    }

    #[test]
    fn should_cancel_a_dictation_on_escape_and_keep_listening() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        spotter.expect_listen_for().return_const(());
        spotter.expect_stop_listening().never();
        let mut paster = MockTextPaster::new();
        paster.expect_paste().never();
        let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
        press(&mut pipeline);
        say(&mut pipeline, "start dictation");
        // When
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Cancel))
            .unwrap();
        // Then
        assert!(!pipeline.is_recording());
        assert!(pipeline.listening());
    }

    #[test]
    fn should_start_listening_as_soon_as_the_pipeline_runs() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        spotter
            .expect_start_listening()
            .with(mockall::predicate::eq(VoiceCommand::STANDBY.to_vec()))
            .once()
            .returning(|_| Ok(()));
        let mut pipeline = pipeline(
            idle_recorder(),
            MockTextPaster::new(),
            spotter,
            RecordingMode::Toggle,
        );
        let (inputs, rx) = std::sync::mpsc::channel::<PipelineInput>();
        drop(inputs);
        // When
        let _ = pipeline.run(rx);
        // Then
        assert!(pipeline.listening());
    }

    #[test]
    fn should_keep_listening_when_a_dictation_cannot_start() {
        // Given the microphone cannot be opened for recording
        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_start()
            .returning(|| Err(anyhow::anyhow!("microphone busy")));
        let mut pipeline = pipeline(
            recorder,
            MockTextPaster::new(),
            lenient_spotter(),
            RecordingMode::Toggle,
        );
        press(&mut pipeline);
        // When
        let segment_start = Instant::now();
        let start = segment_start + PHRASE_LEAD;
        let end = start + PHRASE_LENGTH;
        let results: Vec<anyhow::Result<()>> = [
            SpottingEvent::SpeechStarted(segment_start),
            SpottingEvent::PhraseSpotted {
                phrase: "start dictation".into(),
                start,
                end,
            },
            SpottingEvent::SpeechEnded(end + PHRASE_LEAD),
        ]
        .into_iter()
        .map(|event| pipeline.handle(PipelineInput::Spotting(event)))
        .collect();
        // Then
        assert!(results.iter().all(|r| r.is_ok()), "{results:?}");
        assert!(!pipeline.is_recording());
        assert!(pipeline.listening());
    }
}
