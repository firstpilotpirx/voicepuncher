//! What the pipeline does when the microphone fails mid-dictation, and what it stops
//! after a failure of its own.

use crate::core::domain::{FailingPart, FailureNotice};

use super::dictation::Dictation;
use super::Pipeline;

impl Pipeline {
    /// The microphone failed or disappeared: a dictation in progress ends as a user stop
    /// would, delivering what was recorded before with the notice that part of it was
    /// lost; outside a dictation the failure is logged only.
    pub(super) fn recording_failed(&mut self, reason: &str) -> anyhow::Result<()> {
        let Some(dictation) = self.dictation.as_mut() else {
            log::info!("microphone failure outside a dictation ignored: {reason}");
            return Ok(());
        };
        log::error!("recording failed: {reason}; the dictation ends here");
        dictation.note_failure(FailureNotice::PartOfDictationLost(FailingPart::Microphone));
        self.stop_recording()
    }

    /// Ends `dictation`, which delivered nothing, on the overlay: a part lost means the
    /// whole dictation was lost.
    pub(super) fn end_empty_cycle_on_overlay(&self, dictation: &mut Dictation) {
        if let Some(FailureNotice::PartOfDictationLost(part)) = dictation.pending_notice {
            dictation.pending_notice = Some(FailureNotice::DictationLost(part));
        }
        self.end_cycle_on_overlay(dictation);
    }

    /// Stops what a pipeline that ended with an error or a panic may have left running:
    /// the recording, listening and the overlay. Each step is guarded, as the panic may have
    /// left a part half-used.
    pub fn stop_after_failure(&mut self) {
        let guarded = |step: &mut dyn FnMut()| {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(step));
        };
        if let Some(session) = self.dictation.take().and_then(|dictation| dictation.live) {
            session.recording_end.signal();
        }
        guarded(&mut || self.stop_recorder_quietly("after a failure"));
        guarded(&mut || self.voice.turn_listening_off());
        guarded(&mut || self.visual_indicator.hide());
        guarded(&mut || self.publish_activity());
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::doubles::transcribing;
    use super::super::tests::PipelineUnderTest;
    use super::super::Pipeline;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::live_transcript_view::HiddenLiveTranscript;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::application::PipelineInput;
    use crate::core::domain::{
        AudioBuffer16k, FailingPart, FailureNotice, PttSignal, RecordingMode,
    };

    const MICROPHONE_ERROR: &str = "the input device reported an error";

    fn recorder(stops: usize, audio: AudioBuffer16k) -> MockAudioRecorder {
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_start().times(..).returning(|| Ok(()));
        recorder
            .expect_stop()
            .times(stops)
            .returning(move || Ok(audio.clone()));
        recorder
            .expect_start_live_chunks()
            .times(..)
            .returning(|_| Ok(None));
        recorder
            .expect_flush_live_chunks()
            .times(..)
            .return_const(());
        recorder
    }

    fn overlay(notice: Option<FailureNotice>) -> MockVisualIndicator {
        let mut overlay = MockVisualIndicator::new();
        overlay.expect_hide().times(..).return_const(());
        overlay
            .expect_live_transcript_view()
            .times(..)
            .returning(|| Box::new(HiddenLiveTranscript));
        match notice {
            Some(notice) => overlay
                .expect_show_failure_notice()
                .once()
                .withf(move |shown| *shown == notice)
                .return_const(()),
            None => overlay.expect_show_failure_notice().never(),
        };
        overlay.expect_show_recording().times(..).return_const(());
        overlay
            .expect_show_waiting_for_focus()
            .times(..)
            .return_const(());
        overlay
            .expect_show_recognizing_speech()
            .times(..)
            .return_const(());
        overlay.expect_show_pasted().times(..).return_const(());
        overlay
            .expect_show_refining_text()
            .times(..)
            .return_const(());
        overlay
    }

    fn pipeline(
        recorder: MockAudioRecorder,
        overlay: MockVisualIndicator,
        mode: RecordingMode,
    ) -> Pipeline {
        PipelineUnderTest::new()
            .recorder(recorder)
            .transcriber(transcribing("hello world"))
            .overlay(overlay)
            .mode(mode)
            .build()
    }

    fn one_second() -> AudioBuffer16k {
        AudioBuffer16k::new(vec![1; 16_000])
    }

    #[test]
    fn should_ignore_a_microphone_failure_outside_a_dictation() {
        let mut pipeline = pipeline(
            recorder(0, one_second()),
            overlay(None),
            RecordingMode::Toggle,
        );
        pipeline
            .handle(PipelineInput::RecordingFailed(MICROPHONE_ERROR.into()))
            .unwrap();
    }

    /// A dictation in `mode` that records `audio` and loses its microphone, expecting `notice`.
    fn expect_notice_mid_dictation(
        audio: AudioBuffer16k,
        notice: FailureNotice,
        mode: RecordingMode,
    ) -> Pipeline {
        let mut pipeline = pipeline(recorder(1, audio), overlay(Some(notice)), mode);
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Pressed))
            .unwrap();
        pipeline
            .handle(PipelineInput::RecordingFailed(MICROPHONE_ERROR.into()))
            .unwrap();
        pipeline
    }

    #[test]
    fn should_end_the_dictation_with_what_was_recorded_and_the_microphone_notice() {
        let notice = FailureNotice::PartOfDictationLost(FailingPart::Microphone);
        expect_notice_mid_dictation(one_second(), notice, RecordingMode::Toggle);
    }

    #[test]
    fn should_announce_the_dictation_lost_when_nothing_was_recorded() {
        let notice = FailureNotice::DictationLost(FailingPart::Microphone);
        expect_notice_mid_dictation(
            AudioBuffer16k::new(Vec::new()),
            notice,
            RecordingMode::Toggle,
        );
    }

    #[test]
    fn should_ignore_the_hold_mode_release_after_the_failure() {
        let notice = FailureNotice::PartOfDictationLost(FailingPart::Microphone);
        let mut pipeline = expect_notice_mid_dictation(one_second(), notice, RecordingMode::Hold);
        pipeline
            .handle(PipelineInput::Hotkey(PttSignal::Released))
            .unwrap();
    }
}
