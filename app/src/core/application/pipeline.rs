use std::sync::mpsc;

use crate::core::application::{
    AudioRecorder, SoundPlayer, TextPaster, TextRefiner, Transcriber, VisualIndicator,
};
use crate::core::domain::{AudioBuffer16k, PttSignal, RecordingMode};

#[derive(Debug, Clone, Copy)]
pub struct PipelineConfig {
    pub min_press_ms: u64,
    pub recording_mode: RecordingMode,
}

#[derive(Debug, Clone, Copy)]
enum PipelineState {
    Idle,
    Recording,
    Transcribing,
    Refining,
    Pasting,
}

pub struct Pipeline {
    pub recorder: Box<dyn AudioRecorder>,
    pub transcriber: Box<dyn Transcriber>,
    pub refiner: Box<dyn TextRefiner>,
    pub paster: Box<dyn TextPaster>,
    pub sound_player: Box<dyn SoundPlayer>,
    pub visual_indicator: Box<dyn VisualIndicator>,
    pub cfg: PipelineConfig,
    state: PipelineState,
}

impl Pipeline {
    pub fn new(
        recorder: Box<dyn AudioRecorder>,
        transcriber: Box<dyn Transcriber>,
        refiner: Box<dyn TextRefiner>,
        paster: Box<dyn TextPaster>,
        sound_player: Box<dyn SoundPlayer>,
        visual_indicator: Box<dyn VisualIndicator>,
        cfg: PipelineConfig,
    ) -> Self {
        Self {
            recorder,
            transcriber,
            refiner,
            paster,
            sound_player,
            visual_indicator,
            cfg,
            state: PipelineState::Idle,
        }
    }

    pub fn run(&mut self, rx: mpsc::Receiver<PttSignal>) -> anyhow::Result<()> {
        loop {
            let signal = rx
                .recv()
                .map_err(|_| anyhow::anyhow!("hotkey listener channel closed"))?;
            self.step(signal)?;
        }
    }

    fn step(&mut self, signal: PttSignal) -> anyhow::Result<()> {
        use PipelineState::*;
        log::info!("pipeline recv {signal:?} in state {:?}", self.state);
        let stops = matches!(
            (self.state, signal, self.cfg.recording_mode),
            (Recording, PttSignal::Released, RecordingMode::Hold)
                | (Recording, PttSignal::Pressed, RecordingMode::Toggle)
        );
        match (self.state, signal) {
            (Idle, PttSignal::Pressed) => {
                self.sound_player.play_start();
                self.recorder.start()?;
                self.visual_indicator.show_recording();
                self.state = Recording;
            }
            (Recording, _) if stops => {
                self.sound_player.play_stop();
                self.visual_indicator.show_processing();
                let audio = self.recorder.stop()?;
                self.state = Idle;
                if audio.is_empty() || audio.duration_ms() < self.cfg.min_press_ms {
                    log::info!("dropped: too short ({} ms)", audio.duration_ms());
                    self.visual_indicator.hide();
                    return Ok(());
                }
                self.run_transcribe_refine_paste(audio);
            }
            (Recording, PttSignal::Cancel) => {
                let _ = self.recorder.stop()?;
                self.sound_player.play_cancel();
                self.visual_indicator.hide();
                self.state = Idle;
                log::info!("recording cancelled");
            }
            (s, sig) => log::warn!("ignoring {sig:?} in state {s:?}"),
        }
        Ok(())
    }

    pub fn run_once(&mut self, audio: AudioBuffer16k) -> anyhow::Result<()> {
        let raw = self.transcriber.transcribe(&audio)?;
        let cleaned = self.refiner.refine(&raw)?;
        self.paster.paste(&cleaned)?;
        eprintln!("{} {}", console::style("▸").dim(), cleaned.as_str());
        Ok(())
    }

    fn run_transcribe_refine_paste(&mut self, audio: AudioBuffer16k) {
        self.state = PipelineState::Transcribing;
        log::info!("transcribing…");
        let raw = match self.transcriber.transcribe(&audio) {
            Err(e) => {
                log::warn!("transcription failed, dropping cycle: {e:#}");
                self.state = PipelineState::Idle;
                self.visual_indicator.hide();
                return;
            }
            Ok(r) => r,
        };

        self.state = PipelineState::Refining;
        log::info!("refining…");
        let cleaned = match self.refiner.refine(&raw) {
            Err(e) => {
                log::error!("refinement failed, dropping cycle: {e:#}");
                self.state = PipelineState::Idle;
                self.visual_indicator.hide();
                return;
            }
            Ok(c) => c,
        };

        self.state = PipelineState::Pasting;
        log::info!("pasting…");
        match self.paster.paste(&cleaned) {
            Ok(()) => eprintln!("{} {}", console::style("▸").dim(), cleaned.as_str()),
            Err(e) => log::error!("paste failed, dropping cycle: {e:#}"),
        }
        self.state = PipelineState::Idle;
        self.visual_indicator.hide();
        log::info!("done");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::sound_player::MockSoundPlayer;
    use crate::core::application::text_paster::MockTextPaster;
    use crate::core::application::text_refiner::MockTextRefiner;
    use crate::core::application::transcriber::MockTranscriber;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::domain::{CleanedText, RawTranscript};
    use anyhow::anyhow;

    fn one_second_audio() -> AudioBuffer16k {
        AudioBuffer16k::new(vec![0i16; 16_000])
    }

    fn noop_sound_player() -> MockSoundPlayer {
        let mut p = MockSoundPlayer::new();
        p.expect_play_start().times(..).return_const(());
        p.expect_play_stop().times(..).return_const(());
        p.expect_play_cancel().times(..).return_const(());
        p
    }

    fn noop_visual_indicator() -> MockVisualIndicator {
        let mut v = MockVisualIndicator::new();
        v.expect_show_recording().times(..).return_const(());
        v.expect_show_processing().times(..).return_const(());
        v.expect_hide().times(..).return_const(());
        v
    }

    fn make_pipeline(
        recorder: MockAudioRecorder,
        transcriber: MockTranscriber,
        refiner: MockTextRefiner,
        paster: MockTextPaster,
    ) -> Pipeline {
        make_pipeline_with_mode(
            recorder,
            transcriber,
            refiner,
            paster,
            RecordingMode::Toggle,
        )
    }

    fn make_pipeline_with_mode(
        recorder: MockAudioRecorder,
        transcriber: MockTranscriber,
        refiner: MockTextRefiner,
        paster: MockTextPaster,
        recording_mode: RecordingMode,
    ) -> Pipeline {
        Pipeline::new(
            Box::new(recorder),
            Box::new(transcriber),
            Box::new(refiner),
            Box::new(paster),
            Box::new(noop_sound_player()),
            Box::new(noop_visual_indicator()),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode,
            },
        )
    }

    #[test]
    fn should_start_in_idle_state() {
        // Given
        let pipeline = make_pipeline(
            MockAudioRecorder::new(),
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        // (no signals sent)
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_transition_idle_to_recording_on_pressed() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Recording));
    }

    #[test]
    fn should_drop_cycle_when_audio_too_short() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(AudioBuffer16k::new(vec![0i16; 10])));
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_complete_full_pipeline_and_return_to_idle() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(RawTranscript::new("hello world".to_owned()).unwrap()));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello world".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));

        let mut pipeline = make_pipeline(recorder, transcriber, refiner, paster);
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_drop_cycle_and_not_paste_when_transcription_fails() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Err(anyhow!("whisper failure")));

        let mut pipeline = make_pipeline(
            recorder,
            transcriber,
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_drop_cycle_and_not_paste_when_refinement_fails() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(RawTranscript::new("hello".to_owned()).unwrap()));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Err(anyhow!("llm failure")));

        let mut pipeline = make_pipeline(recorder, transcriber, refiner, MockTextPaster::new());
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_drop_cycle_when_paste_fails() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(RawTranscript::new("hello".to_owned()).unwrap()));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster
            .expect_paste()
            .once()
            .returning(|_| Err(anyhow!("accessibility denied")));

        let mut pipeline = make_pipeline(recorder, transcriber, refiner, paster);
        pipeline.step(PttSignal::Pressed).unwrap();
        // When — second Pressed stops in Toggle mode
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_stop_recording_on_released_in_hold_mode() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(RawTranscript::new("hello".to_owned()).unwrap()));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));

        let mut pipeline =
            make_pipeline_with_mode(recorder, transcriber, refiner, paster, RecordingMode::Hold);
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Released).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_warn_and_stay_idle_on_released_while_idle() {
        // Given
        let mut pipeline = make_pipeline(
            MockAudioRecorder::new(),
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        pipeline.step(PttSignal::Released).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_run_loop_until_channel_closed() {
        // Given
        let (tx, rx) = mpsc::channel::<PttSignal>();
        drop(tx);

        let mut pipeline = make_pipeline(
            MockAudioRecorder::new(),
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        let result = pipeline.run(rx);
        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("channel closed"));
    }

    #[test]
    fn should_ignore_pressed_signal_while_transcribing() {
        // Given
        let mut pipeline = make_pipeline(
            MockAudioRecorder::new(),
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.state = PipelineState::Transcribing;
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Transcribing));
    }

    #[test]
    fn should_ignore_released_signal_while_pasting() {
        // Given
        let mut pipeline = make_pipeline(
            MockAudioRecorder::new(),
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.state = PipelineState::Pasting;
        // When
        pipeline.step(PttSignal::Released).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Pasting));
    }

    #[test]
    fn should_cancel_recording_and_return_to_idle_on_cancel_signal() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));
        let mut pipeline = make_pipeline(
            recorder,
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then — no transcription, back to Idle
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_call_hide_when_recording_cancelled() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual.expect_show_processing().never();
        visual.expect_hide().once().return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
            },
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then — show_processing never called, hide called once
    }

    #[test]
    fn should_play_cancel_sound_when_recording_cancelled() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut sound = MockSoundPlayer::new();
        sound.expect_play_start().once().return_const(());
        sound.expect_play_stop().never();
        sound.expect_play_cancel().once().return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(sound),
            Box::new(noop_visual_indicator()),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
            },
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then — play_cancel called once, play_stop never called
    }

    #[test]
    fn should_ignore_cancel_signal_while_idle() {
        // Given
        let mut pipeline = make_pipeline(
            MockAudioRecorder::new(),
            MockTranscriber::new(),
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        pipeline.step(PttSignal::Cancel).unwrap();
        // Then
        assert!(matches!(pipeline.state, PipelineState::Idle));
    }

    #[test]
    fn should_run_once_and_return_ok_for_successful_pipeline() {
        // Given
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(RawTranscript::new("hello".to_owned()).unwrap()));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));

        let mut pipeline = make_pipeline(MockAudioRecorder::new(), transcriber, refiner, paster);
        // When
        let result = pipeline.run_once(one_second_audio());
        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_run_once_and_return_err_when_transcription_fails() {
        // Given
        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Err(anyhow::anyhow!("whisper failure")));

        let mut pipeline = make_pipeline(
            MockAudioRecorder::new(),
            transcriber,
            MockTextRefiner::new(),
            MockTextPaster::new(),
        );
        // When
        let result = pipeline.run_once(one_second_audio());
        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_call_show_recording_when_transitioning_to_recording() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual.expect_show_processing().times(..).return_const(());
        visual.expect_hide().times(..).return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
            },
        );
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then — MockVisualIndicator verifies exactly once show_recording was called
    }

    #[test]
    fn should_call_show_processing_when_transcribing_starts() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(one_second_audio()));

        let mut transcriber = MockTranscriber::new();
        transcriber
            .expect_transcribe()
            .once()
            .returning(|_| Ok(RawTranscript::new("hello".to_owned()).unwrap()));

        let mut refiner = MockTextRefiner::new();
        refiner
            .expect_refine()
            .once()
            .returning(|_| Ok(CleanedText::new("hello".to_owned()).unwrap()));

        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual.expect_show_processing().once().return_const(());
        visual.expect_hide().once().return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(transcriber),
            Box::new(refiner),
            Box::new(paster),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
            },
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then — MockVisualIndicator verifies the exact call sequence
    }

    #[test]
    fn should_call_hide_when_audio_too_short() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().once().returning(|| Ok(()));
        recorder
            .expect_stop()
            .once()
            .returning(|| Ok(AudioBuffer16k::new(vec![0i16; 10])));

        let mut visual = MockVisualIndicator::new();
        visual.expect_show_recording().once().return_const(());
        visual.expect_show_processing().once().return_const(());
        visual.expect_hide().once().return_const(());

        let mut pipeline = Pipeline::new(
            Box::new(recorder),
            Box::new(MockTranscriber::new()),
            Box::new(MockTextRefiner::new()),
            Box::new(MockTextPaster::new()),
            Box::new(noop_sound_player()),
            Box::new(visual),
            PipelineConfig {
                min_press_ms: 200,
                recording_mode: RecordingMode::Toggle,
            },
        );
        pipeline.step(PttSignal::Pressed).unwrap();
        // When
        pipeline.step(PttSignal::Pressed).unwrap();
        // Then — show_processing fires immediately on stop, hide fires after too-short check
    }
}
