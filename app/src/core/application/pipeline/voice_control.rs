use std::time::Instant;

use crate::core::application::{CommandSpotter, KeyPresser, PipelineInput};
use crate::core::domain::{
    AudioBuffer16k, CommandPlacement, SpokenCommand, SpottingEvent, VoiceCommand,
    VoiceCommandRecognizer,
};

use super::{Pipeline, StartCue};

pub(super) struct VoiceControl {
    spotter: Box<dyn CommandSpotter>,
    key_presser: Box<dyn KeyPresser>,
    recognizer: VoiceCommandRecognizer,
    listening: bool,
    /// The command spotting service is away: listening resumes once it is back.
    away: bool,
    /// Where the last command carried out ended.
    last_carried_end: Option<Instant>,
    /// Commands ending by then were carried out before the service went away: the audio
    /// sent again to the service that is back may report them a second time.
    carried_before_outage: Option<Instant>,
}

impl Pipeline {
    /// Turns voice control on: listening runs alongside push-to-talk, and spoken commands
    /// drive dictations and key presses as the hotkey does.
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
            away: false,
            last_carried_end: None,
            carried_before_outage: None,
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
            PipelineInput::Hotkey(signal) => self.step(signal),
            PipelineInput::Spotting(event) => {
                for spoken in self.recognize(event) {
                    self.carry_out_or_log(spoken);
                }
                Ok(())
            }
            PipelineInput::SpottingLost => self.lose_spotting(),
            PipelineInput::SpottingBack => self.regain_spotting(),
        }
    }

    fn voice(&mut self) -> &mut VoiceControl {
        self.voice.as_mut().expect("voice control is on")
    }

    fn listening(&self) -> bool {
        self.voice.as_ref().is_some_and(|voice| voice.listening)
    }

    /// Points the service at the phrases of a running dictation or of standby, whatever
    /// started or ended the dictation; nothing without voice control or while not listening.
    pub(super) fn follow_dictation(&mut self, dictating: bool) {
        let Some(voice) = self.voice.as_mut().filter(|voice| voice.listening) else {
            return;
        };
        voice.spotter.listen_for(if dictating {
            &VoiceCommand::DICTATING
        } else {
            &VoiceCommand::STANDBY
        });
    }

    /// The sound heard since `since`; none without voice control.
    pub(super) fn heard_since(&self, since: Instant) -> AudioBuffer16k {
        self.voice.as_ref().map_or_else(
            || AudioBuffer16k::new(Vec::new()),
            |voice| voice.spotter.heard_since(since),
        )
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

    fn recognize(&mut self, event: SpottingEvent) -> Vec<SpokenCommand> {
        if !self.listening() {
            return Vec::new();
        }
        self.voice().recognizer.feed(event)
    }

    /// A command that fails (the microphone is busy, a key press is refused) is logged;
    /// the app keeps listening for the next one.
    fn carry_out_or_log(&mut self, spoken: SpokenCommand) {
        let voice = self.voice();
        if voice
            .carried_before_outage
            .is_some_and(|until| spoken.ends_at <= until)
        {
            log::info!(
                "voice control: \"{}\" was carried out before the outage, skipped",
                spoken.command.phrase()
            );
            return;
        }
        voice.last_carried_end = Some(
            voice
                .last_carried_end
                .map_or(spoken.ends_at, |end| end.max(spoken.ends_at)),
        );
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
            VoiceCommand::StartDictation => self.start_dictation_after(spoken, StartCue::Start),
            VoiceCommand::StopDictation => {
                self.command_words.closing = spoken.placement != CommandPlacement::Alone;
                self.recorder.truncate_at(spoken.started_at);
                self.stop_recording()
            }
            VoiceCommand::PressEnter if self.is_recording() => {
                // "enter" during a dictation is "stop enter": paste, then send.
                self.command_words.closing = spoken.placement != CommandPlacement::Alone;
                self.recorder.truncate_at(spoken.started_at);
                self.stop_recording()?;
                self.press_keys(VoiceCommand::PressEnter)
            }
            VoiceCommand::CancelDictation => self.cancel_recording(),
            VoiceCommand::RestartDictation => {
                // One restart cue instead of the cancel and start cues, so it is heard apart.
                self.discard_recording()?;
                self.start_dictation_after(spoken, StartCue::Restart)?;
                self.confirm(VoiceCommand::RestartDictation);
                Ok(())
            }
            VoiceCommand::PressEnter | VoiceCommand::ClearAllText => {
                self.press_keys(spoken.command)
            }
        }
    }

    /// Starts a dictation that begins where `spoken` ended, so the speech said right after
    /// the command, before it was recognized, is part of it.
    fn start_dictation_after(
        &mut self,
        spoken: SpokenCommand,
        cue: StartCue,
    ) -> anyhow::Result<()> {
        self.start_recording_from(Some(spoken.ends_at), cue)?;
        // Only an end the voice activity detector placed lies safely past the last word.
        self.command_words.opening =
            (spoken.placement != CommandPlacement::Alone).then_some(spoken.command);
        Ok(())
    }

    /// Presses the keys of "press enter" or "clear all text", then confirms the command.
    fn press_keys(&mut self, command: VoiceCommand) -> anyhow::Result<()> {
        let keys = self.voice().key_presser.as_mut();
        let pressed = match command {
            VoiceCommand::ClearAllText => keys.clear_all_text(),
            _ => keys.press_enter(),
        };
        match pressed {
            Ok(()) => self.confirm(command),
            Err(e) => log::error!("voice control: key press failed: {e:#}"),
        }
        Ok(())
    }

    /// The sound and the command sign of a command with no visible effect of its own.
    fn confirm(&self, command: VoiceCommand) {
        match command {
            VoiceCommand::PressEnter => self.sound_player.play_enter(),
            VoiceCommand::ClearAllText => self.sound_player.play_clear(),
            VoiceCommand::RestartDictation => self.sound_player.play_restart(),
            _ => return,
        }
        self.visual_indicator.show_command_sign(command);
    }

    /// The service went away: listening stays on, a running dictation keeps recording, and
    /// the adapter reaches for the service until it is back.
    fn lose_spotting(&mut self) -> anyhow::Result<()> {
        if !self.listening() || self.voice().away {
            return Ok(());
        }
        log::error!(
            "voice control: the command spotting service stopped answering; listening resumes once it is back"
        );
        let recording = self.is_recording();
        let voice = self.voice();
        voice.away = true;
        voice.carried_before_outage = voice.last_carried_end;
        self.sound_player.play_listening_off();
        log::info!(
            "voice control: listening → away{}",
            if recording {
                ", the dictation keeps recording"
            } else {
                ""
            }
        );
        Ok(())
    }

    /// The service is back: the commands of the current state are spotted again.
    fn regain_spotting(&mut self) -> anyhow::Result<()> {
        if !self.listening() || !self.voice().away {
            return Ok(());
        }
        let recording = self.is_recording();
        let voice = self.voice();
        voice.away = false;
        voice.recognizer = VoiceCommandRecognizer::new();
        self.follow_dictation(recording);
        self.sound_player.play_listening_on();
        log::info!(
            "voice control: away → listening again{}",
            if recording {
                " during the dictation"
            } else {
                ""
            }
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::command_spotter::MockCommandSpotter;
    use crate::core::application::key_presser::MockKeyPresser;
    use crate::core::application::live_transcript_view::HiddenLiveTranscript;
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
    /// Half a second of sound at 16 kHz.
    const LEAD_IN_SAMPLES: usize = 8_000;

    fn sounds() -> MockSoundPlayer {
        let mut sounds = MockSoundPlayer::new();
        sounds.expect_play_start().return_const(());
        sounds.expect_play_stop().return_const(());
        sounds.expect_play_cancel().return_const(());
        sounds.expect_play_listening_on().return_const(());
        sounds.expect_play_listening_off().return_const(());
        sounds.expect_play_enter().return_const(());
        sounds.expect_play_clear().return_const(());
        sounds.expect_play_restart().return_const(());
        sounds
    }

    fn overlay() -> MockVisualIndicator {
        let mut overlay = MockVisualIndicator::new();
        overlay.expect_show_recording().return_const(());
        overlay.expect_show_recognizing_speech().return_const(());
        overlay.expect_show_refining_text().return_const(());
        overlay.expect_hide().return_const(());
        overlay.expect_show_command_sign().return_const(());
        overlay
            .expect_live_transcript_view()
            .returning(|| Box::new(HiddenLiveTranscript));
        overlay
    }

    fn idle_recorder() -> MockAudioRecorder {
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().never();
        recorder
    }

    fn recording_recorder() -> MockAudioRecorder {
        let mut recorder = recording_recorder_without_lead_in();
        recorder.expect_begin_with().return_const(());
        recorder
    }

    fn recording_recorder_without_lead_in() -> MockAudioRecorder {
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

    fn listening_spotter() -> MockCommandSpotter {
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        spotter.expect_listen_for().return_const(());
        spotter.expect_stop_listening().return_const(());
        spotter
    }

    fn lenient_spotter() -> MockCommandSpotter {
        let mut spotter = listening_spotter();
        hears_nothing(&mut spotter);
        spotter
    }

    fn hears_nothing(spotter: &mut MockCommandSpotter) {
        spotter
            .expect_heard_since()
            .returning(|_| AudioBuffer16k::new(Vec::new()));
    }

    fn said(command: VoiceCommand, placement: CommandPlacement, start: Instant) -> PipelineInput {
        PipelineInput::Spotting(SpottingEvent::CommandsSaid {
            commands: vec![command],
            start,
            end: start + PHRASE_LENGTH,
            placement,
        })
    }

    fn pipeline(
        recorder: MockAudioRecorder,
        paster: MockTextPaster,
        spotter: MockCommandSpotter,
        mode: RecordingMode,
    ) -> Pipeline {
        pipeline_with(
            recorder,
            paster,
            spotter,
            sounds(),
            MockKeyPresser::new(),
            mode,
        )
    }

    /// Toggle recording, transcribed in one go when it ends.
    fn toggle_cfg() -> PipelineConfig {
        PipelineConfig {
            min_press_ms: 0,
            recording_mode: RecordingMode::Toggle,
            chunking: ChunkingCfg {
                chunk_threshold_secs: 0,
                ..ChunkingCfg::default()
            },
        }
    }

    fn pipeline_with(
        recorder: MockAudioRecorder,
        paster: MockTextPaster,
        spotter: MockCommandSpotter,
        sounds: MockSoundPlayer,
        keys: MockKeyPresser,
        mode: RecordingMode,
    ) -> Pipeline {
        let cfg = PipelineConfig {
            recording_mode: mode,
            ..toggle_cfg()
        };
        Pipeline::new(
            Box::new(recorder),
            Box::new(answering_transcriber()),
            Box::new(passthrough_refiner()),
            Box::new(paster),
            Box::new(sounds),
            Box::new(overlay()),
            cfg,
        )
        .with_voice_control(Box::new(spotter), Box::new(keys))
    }

    /// A pipeline that transcribes a dictation in live chunks while it runs.
    fn live_chunking_pipeline(
        recorder: MockAudioRecorder,
        spotter: MockCommandSpotter,
    ) -> Pipeline {
        let cfg = PipelineConfig {
            min_press_ms: 0,
            recording_mode: RecordingMode::Toggle,
            chunking: ChunkingCfg {
                chunk_threshold_secs: 1,
                ..ChunkingCfg::default()
            },
        };
        Pipeline::new(
            Box::new(recorder),
            Box::new(answering_transcriber()),
            Box::new(passthrough_refiner()),
            Box::new(MockTextPaster::new()),
            Box::new(sounds()),
            Box::new(overlay()),
            cfg,
        )
        .with_voice_control(Box::new(spotter), Box::new(MockKeyPresser::new()))
    }

    fn listen(pipeline: &mut Pipeline) {
        pipeline.turn_listening_on();
    }

    fn press(pipeline: &mut Pipeline) {
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
    }

    /// Says `phrase` on its own and returns the moment the phrase started.
    fn say(pipeline: &mut Pipeline, phrase: &str) -> Instant {
        say_at(pipeline, phrase, Instant::now())
    }

    /// Says `phrase` on its own in a segment starting at `segment_start`.
    fn say_at(pipeline: &mut Pipeline, phrase: &str, segment_start: Instant) -> Instant {
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
    fn should_record_with_the_hotkey_and_keep_listening() {
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
        spotter.expect_stop_listening().never();
        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));
        let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
        listen(&mut pipeline);
        // When
        press(&mut pipeline);
        press(&mut pipeline);
        // Then
        assert!(!pipeline.is_recording());
        assert!(pipeline.listening());
    }

    #[test]
    fn should_end_a_hold_dictation_on_release_and_keep_listening() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        spotter.expect_listen_for().return_const(());
        spotter.expect_stop_listening().never();
        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));
        let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Hold);
        listen(&mut pipeline);
        press(&mut pipeline);
        // When
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Released))
            .unwrap();
        // Then
        assert!(!pipeline.is_recording());
        assert!(pipeline.listening());
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
        listen(&mut pipeline);
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
        hears_nothing(&mut spotter);
        let mut recorder = recording_recorder();
        recorder.expect_truncate_at().return_const(());
        let mut paster = MockTextPaster::new();
        paster.expect_paste().returning(|_| Ok(()));
        let mut pipeline = pipeline(recorder, paster, spotter, RecordingMode::Toggle);
        listen(&mut pipeline);
        // When
        say(&mut pipeline, "start dictation");
        say(&mut pipeline, "stop dictation");
        // Then the mock expectations hold
    }

    #[test]
    fn should_keep_recording_and_listening_when_the_service_goes_away_during_a_dictation() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        spotter.expect_listen_for().return_const(());
        spotter.expect_stop_listening().never();
        hears_nothing(&mut spotter);
        let mut paster = MockTextPaster::new();
        paster.expect_paste().never();
        let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
        listen(&mut pipeline);
        say(&mut pipeline, "start dictation");
        // When
        pipeline.handle(PipelineInput::SpottingLost).unwrap();
        // Then
        assert!(pipeline.is_recording());
    }

    #[test]
    fn should_listen_for_the_commands_of_a_running_dictation_when_the_service_is_back() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        let mut order = mockall::Sequence::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        hears_nothing(&mut spotter);
        spotter
            .expect_listen_for()
            .with(mockall::predicate::eq(VoiceCommand::DICTATING.to_vec()))
            .times(2)
            .in_sequence(&mut order)
            .return_const(());
        let mut paster = MockTextPaster::new();
        paster.expect_paste().never();
        let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
        listen(&mut pipeline);
        say(&mut pipeline, "start dictation");
        pipeline.handle(PipelineInput::SpottingLost).unwrap();
        // When
        pipeline.handle(PipelineInput::SpottingBack).unwrap();
        // Then the mock expectations hold
    }

    #[test]
    fn should_cancel_a_dictation_on_escape_and_keep_listening() {
        // Given
        let mut spotter = MockCommandSpotter::new();
        let mut order = mockall::Sequence::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
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
        spotter.expect_stop_listening().never();
        hears_nothing(&mut spotter);
        let mut paster = MockTextPaster::new();
        paster.expect_paste().never();
        let mut pipeline = pipeline(recording_recorder(), paster, spotter, RecordingMode::Toggle);
        listen(&mut pipeline);
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
        let listened = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        let seen = std::sync::Arc::clone(&listened);
        spotter
            .expect_listen_for()
            .returning(move |commands| seen.lock().unwrap().push(commands.to_vec()));
        let mut pipeline = pipeline(
            recorder,
            MockTextPaster::new(),
            spotter,
            RecordingMode::Toggle,
        );
        listen(&mut pipeline);
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
        assert_eq!(
            *listened.lock().unwrap(),
            [
                VoiceCommand::DICTATING.to_vec(),
                VoiceCommand::STANDBY.to_vec()
            ]
        );
    }

    #[test]
    fn should_listen_for_the_dictation_phrases_before_the_recording_starts() {
        // Given
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut spotter = MockCommandSpotter::new();
        spotter.expect_start_listening().returning(|_| Ok(()));
        let seen = std::sync::Arc::clone(&order);
        spotter
            .expect_listen_for()
            .returning(move |commands| seen.lock().unwrap().push(format!("listen {commands:?}")));
        hears_nothing(&mut spotter);
        let mut recorder = MockAudioRecorder::new();
        let seen = std::sync::Arc::clone(&order);
        recorder.expect_start().returning(move || {
            seen.lock().unwrap().push("record".to_owned());
            Ok(())
        });
        recorder.expect_begin_with().return_const(());
        let mut pipeline = pipeline(
            recorder,
            MockTextPaster::new(),
            spotter,
            RecordingMode::Toggle,
        );
        listen(&mut pipeline);
        // When
        say(&mut pipeline, "start dictation");
        // Then
        let order = order.lock().unwrap();
        assert_eq!(
            order.first().map(String::as_str),
            Some(format!("listen {:?}", VoiceCommand::DICTATING).as_str())
        );
        assert_eq!(order.get(1).map(String::as_str), Some("record"));
    }

    /// A spotter that heard half a second since `end`, and nothing since any other moment.
    fn heard_half_a_second_since(end: Instant) -> MockCommandSpotter {
        let mut spotter = listening_spotter();
        spotter.expect_heard_since().returning(move |since| {
            let samples = if since == end { LEAD_IN_SAMPLES } else { 0 };
            AudioBuffer16k::new(vec![0; samples])
        });
        spotter
    }

    /// A recorder that notes the length of every lead-in it begins with.
    fn noting_lead_ins(lead_ins: &Arc<Mutex<Vec<usize>>>) -> MockAudioRecorder {
        let mut recorder = recording_recorder_without_lead_in();
        let seen = Arc::clone(lead_ins);
        recorder
            .expect_begin_with()
            .returning(move |lead_in| seen.lock().unwrap().push(lead_in.samples().len()));
        recorder
    }

    #[test]
    fn should_begin_a_voice_dictation_where_the_start_ended_wherever_it_was_said() {
        for placement in [
            CommandPlacement::Alone,
            CommandPlacement::Head,
            CommandPlacement::Tail,
            CommandPlacement::Anywhere,
        ] {
            // Given half a second heard since "start" ended
            let start = Instant::now();
            let lead_ins = Arc::new(Mutex::new(Vec::new()));
            let spotter = heard_half_a_second_since(start + PHRASE_LENGTH);
            let recorder = noting_lead_ins(&lead_ins);
            let mut pipeline = pipeline(
                recorder,
                MockTextPaster::new(),
                spotter,
                RecordingMode::Toggle,
            );
            listen(&mut pipeline);
            // When
            let started = pipeline.handle(said(VoiceCommand::StartDictation, placement, start));
            // Then
            assert!(started.is_ok() && pipeline.is_recording(), "{placement:?}");
            assert_eq!(
                *lead_ins.lock().unwrap(),
                [LEAD_IN_SAMPLES],
                "{placement:?}"
            );
        }
    }

    #[test]
    fn should_begin_a_restarted_dictation_where_restart_ended() {
        // Given a dictation runs and half a second was heard since "restart" ended
        let restart_at = Instant::now() + Duration::from_secs(5);
        let lead_ins = Arc::new(Mutex::new(Vec::new()));
        let spotter = heard_half_a_second_since(restart_at + PHRASE_LENGTH);
        let recorder = noting_lead_ins(&lead_ins);
        let mut pipeline = pipeline(
            recorder,
            MockTextPaster::new(),
            spotter,
            RecordingMode::Toggle,
        );
        listen(&mut pipeline);
        say(&mut pipeline, "start dictation");
        // When
        let restarted = pipeline.handle(said(
            VoiceCommand::RestartDictation,
            CommandPlacement::Alone,
            restart_at,
        ));
        // Then
        assert!(restarted.is_ok() && pipeline.is_recording());
        assert_eq!(lead_ins.lock().unwrap().last(), Some(&LEAD_IN_SAMPLES));
    }

    #[test]
    fn should_paste_a_restarted_dictation_without_the_restart_its_lead_in_caught() {
        // Given a dictation runs, and its restart's lead-in caught the end of "restart restart"
        let mut transcriber = MockTranscriber::new();
        transcriber.expect_transcribe().returning(|_| {
            Ok(Some(
                RawTranscript::new("Restart, lunch is at noon.".into()).unwrap(),
            ))
        });
        let pasted = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&pasted);
        let mut paster = MockTextPaster::new();
        paster.expect_paste().returning(move |text| {
            seen.lock().unwrap().push(text.as_str().to_owned());
            Ok(())
        });
        let mut pipeline = Pipeline::new(
            Box::new(recording_recorder()),
            Box::new(transcriber),
            Box::new(passthrough_refiner()),
            Box::new(paster),
            Box::new(sounds()),
            Box::new(overlay()),
            toggle_cfg(),
        )
        .with_voice_control(Box::new(lenient_spotter()), Box::new(MockKeyPresser::new()));
        listen(&mut pipeline);
        say(&mut pipeline, "start dictation");
        let restart = said(
            VoiceCommand::RestartDictation,
            CommandPlacement::Anywhere,
            Instant::now(),
        );
        pipeline.handle(restart).unwrap();
        // When
        press(&mut pipeline);
        // Then
        assert_eq!(*pasted.lock().unwrap(), ["Lunch is at noon."]);
    }

    #[test]
    fn should_take_the_heard_speech_once_the_recorder_runs_and_before_live_chunks() {
        // Given
        let order = Arc::new(Mutex::new(Vec::new()));
        let note = |step: &'static str| {
            let seen = Arc::clone(&order);
            move || seen.lock().unwrap().push(step)
        };
        let mut spotter = listening_spotter();
        let heard = note("take the heard speech");
        spotter.expect_heard_since().returning(move |_| {
            heard();
            AudioBuffer16k::new(Vec::new())
        });
        let mut recorder = MockAudioRecorder::new();
        let started = note("start the recorder");
        recorder.expect_start().returning(move || {
            started();
            Ok(())
        });
        let joined = note("begin with the heard speech");
        recorder.expect_begin_with().returning(move |_| joined());
        let chunked = note("start live chunks");
        recorder.expect_start_live_chunks().returning(move |_| {
            chunked();
            Ok(None)
        });
        let mut pipeline = live_chunking_pipeline(recorder, spotter);
        listen(&mut pipeline);
        // When
        let start = Instant::now();
        let started = pipeline.handle(said(
            VoiceCommand::StartDictation,
            CommandPlacement::Head,
            start,
        ));
        // Then
        assert!(started.is_ok());
        assert_eq!(
            *order.lock().unwrap(),
            [
                "start the recorder",
                "take the heard speech",
                "begin with the heard speech",
                "start live chunks"
            ]
        );
    }

    #[test]
    fn should_paste_and_then_press_return_on_enter_during_a_dictation() {
        // Given
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut recorder = recording_recorder();
        recorder.expect_truncate_at().return_const(());
        let mut paster = MockTextPaster::new();
        let seen = std::sync::Arc::clone(&order);
        paster.expect_paste().once().returning(move |_| {
            seen.lock().unwrap().push("paste");
            Ok(())
        });
        let mut keys = MockKeyPresser::new();
        let seen = std::sync::Arc::clone(&order);
        keys.expect_press_enter().once().returning(move || {
            seen.lock().unwrap().push("return");
            Ok(())
        });
        let mut pipeline = pipeline_with(
            recorder,
            paster,
            lenient_spotter(),
            sounds(),
            keys,
            RecordingMode::Toggle,
        );
        listen(&mut pipeline);
        say(&mut pipeline, "start dictation");
        // When
        say(&mut pipeline, "press enter");
        // Then
        assert!(!pipeline.is_recording());
        assert_eq!(*order.lock().unwrap(), ["paste", "return"]);
    }

    #[test]
    fn should_play_only_the_restart_sound_on_restart() {
        // Given
        let played = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut sounds = MockSoundPlayer::new();
        let seen = std::sync::Arc::clone(&played);
        sounds
            .expect_play_start()
            .returning(move || seen.lock().unwrap().push("start"));
        let seen = std::sync::Arc::clone(&played);
        sounds
            .expect_play_cancel()
            .returning(move || seen.lock().unwrap().push("cancel"));
        let seen = std::sync::Arc::clone(&played);
        sounds
            .expect_play_restart()
            .returning(move || seen.lock().unwrap().push("restart"));
        sounds.expect_play_listening_on().return_const(());
        let mut paster = MockTextPaster::new();
        paster.expect_paste().never();
        let mut pipeline = pipeline_with(
            recording_recorder(),
            paster,
            lenient_spotter(),
            sounds,
            MockKeyPresser::new(),
            RecordingMode::Toggle,
        );
        listen(&mut pipeline);
        say(&mut pipeline, "start dictation");
        // When
        say(&mut pipeline, "restart dictation");
        // Then
        assert!(pipeline.is_recording());
        assert_eq!(*played.lock().unwrap(), ["start", "restart"]);
    }

    #[test]
    fn should_not_carry_out_again_a_command_the_service_reports_twice_across_an_outage() {
        // Given
        let mut keys = MockKeyPresser::new();
        keys.expect_press_enter().once().returning(|| Ok(()));
        let mut pipeline = pipeline_with(
            idle_recorder(),
            MockTextPaster::new(),
            lenient_spotter(),
            sounds(),
            keys,
            RecordingMode::Toggle,
        );
        listen(&mut pipeline);
        let said_at = Instant::now();
        say_at(&mut pipeline, "press enter", said_at);
        pipeline.handle(PipelineInput::SpottingLost).unwrap();
        pipeline.handle(PipelineInput::SpottingBack).unwrap();
        // When the audio sent again reports the same command
        say_at(&mut pipeline, "press enter", said_at);
        // Then the mock expectations hold
    }
}
