use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

use crate::core::application::ports::{AudioRecorder, SoundPlayer, TextPaster, TextRefiner, Transcriber};
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

struct ProcessingGuard<'a>(&'a AtomicBool);

impl Drop for ProcessingGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

pub struct Pipeline {
    pub recorder: Box<dyn AudioRecorder>,
    pub transcriber: Box<dyn Transcriber>,
    pub refiner: Box<dyn TextRefiner>,
    pub paster: Box<dyn TextPaster>,
    pub sound_player: Box<dyn SoundPlayer>,
    pub cfg: PipelineConfig,
    state: PipelineState,
    processing: Arc<AtomicBool>,
}

impl Pipeline {
    pub fn new(
        recorder: Box<dyn AudioRecorder>,
        transcriber: Box<dyn Transcriber>,
        refiner: Box<dyn TextRefiner>,
        paster: Box<dyn TextPaster>,
        sound_player: Box<dyn SoundPlayer>,
        cfg: PipelineConfig,
        processing: Arc<AtomicBool>,
    ) -> Self {
        Self {
            recorder,
            transcriber,
            refiner,
            paster,
            sound_player,
            cfg,
            state: PipelineState::Idle,
            processing,
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
        let stops = matches!(
            (self.state, signal, self.cfg.recording_mode),
            (Recording, PttSignal::Released, RecordingMode::Hold)
                | (Recording, PttSignal::Pressed, RecordingMode::Toggle)
        );
        match (self.state, signal) {
            (Idle, PttSignal::Pressed) => {
                self.sound_player.play_start();
                self.recorder.start()?;
                self.state = Recording;
            }
            (Recording, _) if stops => {
                let audio = self.recorder.stop()?;
                self.sound_player.play_stop();
                self.state = Idle;
                if audio.is_empty() || audio.duration_ms() < self.cfg.min_press_ms {
                    log::info!("dropped: too short ({} ms)", audio.duration_ms());
                    return Ok(());
                }
                self.run_transcribe_refine_paste(audio);
            }
            (s, sig) => log::warn!("ignoring {sig:?} in state {s:?}"),
        }
        Ok(())
    }

    pub fn run_once(&mut self, audio: AudioBuffer16k) -> anyhow::Result<()> {
        let raw = self.transcriber.transcribe(&audio)?;
        let cleaned = self.refiner.refine(&raw)?;
        self.paster.paste(&cleaned)?;
        Ok(())
    }

    fn run_transcribe_refine_paste(&mut self, audio: AudioBuffer16k) {
        self.processing.store(true, Ordering::Relaxed);
        let _guard = ProcessingGuard(&self.processing);
        self.state = PipelineState::Transcribing;
        log::info!("transcribing…");
        let raw = match self.transcriber.transcribe(&audio) {
            Err(e) => {
                log::warn!("transcription failed, dropping cycle: {e:#}");
                self.state = PipelineState::Idle;
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
                return;
            }
            Ok(c) => c,
        };

        self.state = PipelineState::Pasting;
        log::info!("pasting…");
        if let Err(e) = self.paster.paste(&cleaned) {
            log::error!("paste failed, dropping cycle: {e:#}");
        }
        self.state = PipelineState::Idle;
        log::info!("done");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::AtomicBool, mpsc};

    use super::*;
    use crate::core::application::ports::audio_recorder::MockAudioRecorder;
    use crate::core::application::ports::sound_player::MockSoundPlayer;
    use crate::core::application::ports::text_paster::MockTextPaster;
    use crate::core::application::ports::text_refiner::MockTextRefiner;
    use crate::core::application::ports::transcriber::MockTranscriber;
    use crate::core::domain::{CleanedText, RawTranscript};
    use anyhow::anyhow;

    fn one_second_audio() -> AudioBuffer16k {
        AudioBuffer16k::new(vec![0i16; 16_000])
    }

    fn noop_sound_player() -> MockSoundPlayer {
        let mut p = MockSoundPlayer::new();
        p.expect_play_start().times(..).return_const(());
        p.expect_play_stop().times(..).return_const(());
        p
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
            PipelineConfig {
                min_press_ms: 200,
                recording_mode,
            },
            Arc::new(AtomicBool::new(false)),
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
}
