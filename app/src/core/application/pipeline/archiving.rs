use std::time::{Instant, SystemTime};

use crate::core::application::{
    ArchiveEntry, DictationArchive, DictationOutcome, DictationStart, Insertion, RefinementOutcome,
    StageTiming,
};
use crate::core::domain::{AudioBuffer16k, CleanedText, RawTranscript, RecordingMode};

use super::dictation::Dictation;
use super::Pipeline;

/// What is known so far of the running dictation, for its archive entry.
pub(super) struct DictationTrace {
    started_at: SystemTime,
    start: DictationStart,
    /// None until known: a dictation transcribed in chunks takes it from the recorder at its end.
    audio: Option<AudioBuffer16k>,
    chunk_texts: Vec<String>,
    raw_transcript: Option<String>,
    pasted_text: Option<String>,
    refinement: Option<RefinementOutcome>,
    lost_chunks: usize,
}

impl DictationTrace {
    pub(super) fn new(start: DictationStart) -> Self {
        Self {
            started_at: SystemTime::now(),
            start,
            audio: None,
            chunk_texts: Vec::new(),
            raw_transcript: None,
            pasted_text: None,
            refinement: None,
            lost_chunks: 0,
        }
    }

    fn into_entry(
        self,
        audio: AudioBuffer16k,
        outcome: DictationOutcome,
        timings: &[StageTiming],
    ) -> ArchiveEntry {
        ArchiveEntry {
            started_at: self.started_at,
            start: self.start,
            audio,
            chunk_texts: self.chunk_texts,
            raw_transcript: self.raw_transcript,
            pasted_text: self.pasted_text,
            refinement: self.refinement,
            outcome,
            lost_chunks: self.lost_chunks,
            timings: timings.to_vec(),
        }
    }
}

/// How a recording started: by voice when it begins at a spoken command, else by the hotkey.
pub(super) fn dictation_start(begins_at: Option<Instant>, mode: RecordingMode) -> DictationStart {
    match (begins_at, mode) {
        (Some(_), _) => DictationStart::Voice,
        (None, RecordingMode::Hold) => DictationStart::HotkeyHeld,
        (None, RecordingMode::Toggle) => DictationStart::HotkeyToggled,
    }
}

/// The outcome of a dictation whose text went to the paster.
pub(super) fn insertion_outcome(insertion: &anyhow::Result<Insertion>) -> DictationOutcome {
    match insertion {
        Ok(Insertion::Pasted | Insertion::PastedAfterWait) => DictationOutcome::Pasted,
        Ok(Insertion::LeftInClipboard(_)) => DictationOutcome::LeftInClipboard,
        Err(_) => DictationOutcome::PasteFailed,
    }
}

/// The outcome of a dictation whose speech recognition returned no text.
pub(super) fn no_text_outcome(lost_chunks: usize) -> DictationOutcome {
    if lost_chunks > 0 {
        DictationOutcome::SpeechRecognitionFailed
    } else {
        DictationOutcome::NoText
    }
}

impl Dictation {
    pub(super) fn trace_audio(&mut self, audio: AudioBuffer16k) {
        if let Some(trace) = self.trace.as_mut() {
            trace.audio = Some(audio);
        }
    }

    /// The text speech recognition returned for each chunk, and the chunks it lost.
    pub(super) fn trace_chunks(&mut self, texts: &[String], lost_chunks: usize) {
        if let Some(trace) = self.trace.as_mut() {
            trace.chunk_texts = texts.to_vec();
            trace.lost_chunks = lost_chunks;
        }
    }

    /// What went to text refinement and what came back to paste, if anything; `refining`
    /// says whether text refinement was on.
    pub(super) fn trace_refinement(
        &mut self,
        raw: &RawTranscript,
        pasted: Option<(&CleanedText, bool)>,
        refining: bool,
    ) {
        let Some(trace) = self.trace.as_mut() else {
            return;
        };
        trace.raw_transcript = Some(raw.as_str().to_owned());
        let Some((text, fell_back)) = pasted else {
            return;
        };
        trace.pasted_text = Some(text.as_str().to_owned());
        trace.refinement = Some(match (refining, fell_back) {
            (false, _) => RefinementOutcome::Off,
            (true, true) => RefinementOutcome::FellBack,
            (true, false) => RefinementOutcome::Refined,
        });
    }
}

impl Pipeline {
    /// Keeps every dictation that reached speech recognition in `archive`.
    pub fn with_archive(mut self, archive: Box<dyn DictationArchive>) -> Self {
        self.sinks.archive = Some(archive);
        self
    }

    /// Ends `dictation` with `outcome` and keeps it in the archive; a failure to keep it is
    /// only logged, so it never costs the dictation.
    pub(super) fn close_trace(
        &mut self,
        dictation: Dictation,
        outcome: DictationOutcome,
        timings: &[StageTiming],
    ) {
        let Some(mut trace) = dictation.trace else {
            return;
        };
        let audio = match trace.audio.take() {
            Some(audio) => audio,
            None => self
                .recorder
                .recording()
                .unwrap_or_else(|| AudioBuffer16k::new(Vec::new())),
        };
        let entry = trace.into_entry(audio, outcome, timings);
        if let Some(archive) = self.sinks.archive.as_mut() {
            if let Err(e) = archive.keep(&entry) {
                log::warn!("dictation not archived: {e:#}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::PipelineUnderTest;
    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::dictation_archive::MockDictationArchive;
    use crate::core::application::WaitEnd;

    fn pipeline_over(recorder: MockAudioRecorder) -> Pipeline {
        PipelineUnderTest::new().recorder(recorder).build()
    }

    #[test]
    fn should_start_by_voice_when_the_recording_begins_at_a_command() {
        // Given
        let begins_at = Some(Instant::now());
        // When
        let start = dictation_start(begins_at, RecordingMode::Hold);
        // Then
        assert_eq!(start, DictationStart::Voice);
    }

    #[test]
    fn should_start_by_the_hotkey_in_its_recording_mode() {
        // Given / When
        let held = dictation_start(None, RecordingMode::Hold);
        let toggled = dictation_start(None, RecordingMode::Toggle);
        // Then
        assert_eq!(held, DictationStart::HotkeyHeld);
        assert_eq!(toggled, DictationStart::HotkeyToggled);
    }

    #[test]
    fn should_map_every_insertion_to_its_outcome() {
        // Given
        let cases = [
            (Ok(Insertion::Pasted), DictationOutcome::Pasted),
            (Ok(Insertion::PastedAfterWait), DictationOutcome::Pasted),
            (
                Ok(Insertion::LeftInClipboard(WaitEnd::TimedOut)),
                DictationOutcome::LeftInClipboard,
            ),
            (
                Err(anyhow::anyhow!("no paste")),
                DictationOutcome::PasteFailed,
            ),
        ];
        for (insertion, expected) in cases {
            // When
            let outcome = insertion_outcome(&insertion);
            // Then
            assert_eq!(outcome, expected);
        }
    }

    #[test]
    fn should_call_a_dictation_without_text_failed_when_chunks_were_lost() {
        // Given / When
        let lost = no_text_outcome(1);
        let silent = no_text_outcome(0);
        // Then
        assert_eq!(lost, DictationOutcome::SpeechRecognitionFailed);
        assert_eq!(silent, DictationOutcome::NoText);
    }

    #[test]
    fn should_build_the_entry_from_the_trace() {
        // Given
        let mut trace = DictationTrace::new(DictationStart::HotkeyHeld);
        trace.chunk_texts = vec!["hello".to_owned()];
        trace.lost_chunks = 1;
        let audio = AudioBuffer16k::new(vec![7; 16]);
        // When
        let entry = trace.into_entry(audio, DictationOutcome::NoText, &[]);
        // Then
        assert_eq!(entry.start, DictationStart::HotkeyHeld);
        assert_eq!(entry.chunk_texts, ["hello"]);
        assert_eq!(entry.lost_chunks, 1);
        assert_eq!(entry.audio.samples(), [7; 16]);
        assert_eq!(entry.outcome, DictationOutcome::NoText);
    }

    #[test]
    fn should_trace_nothing_without_an_archive() {
        // Given, then: the recorder is never asked for its recording
        let mut recorder = MockAudioRecorder::new();
        recorder.expect_recording().never();
        let mut pipeline = pipeline_over(recorder);
        let dictation = pipeline.new_dictation(DictationStart::HotkeyToggled);
        // When
        pipeline.close_trace(dictation, DictationOutcome::Pasted, &[]);
    }

    #[test]
    fn should_keep_a_closed_dictation_with_the_whole_recording() {
        // Given
        let mut recorder = MockAudioRecorder::new();
        recorder
            .expect_recording()
            .once()
            .returning(|| Some(AudioBuffer16k::new(vec![5; 32])));
        let mut archive = MockDictationArchive::new();
        archive
            .expect_keep()
            .withf(|entry| {
                entry.outcome == DictationOutcome::LeftInClipboard
                    && entry.audio.samples() == [5; 32]
                    && entry.timings == [StageTiming { stage: "paste", ms: 9 }]
            })
            .once()
            .returning(|_| Ok(()));
        let mut pipeline = pipeline_over(recorder).with_archive(Box::new(archive));
        let dictation = pipeline.new_dictation(DictationStart::HotkeyToggled);
        // When
        pipeline.close_trace(
            dictation,
            DictationOutcome::LeftInClipboard,
            &[StageTiming { stage: "paste", ms: 9 }],
        );
        // Then the archive got the entry
    }

    #[test]
    fn should_go_on_when_the_archive_fails() {
        // Given
        let mut archive = MockDictationArchive::new();
        archive
            .expect_keep()
            .once()
            .returning(|_| Err(anyhow::anyhow!("disk full")));
        let mut pipeline = pipeline_over(MockAudioRecorder::new()).with_archive(Box::new(archive));
        let mut dictation = pipeline.new_dictation(DictationStart::Voice);
        dictation.trace_audio(AudioBuffer16k::new(vec![1; 16]));
        // When
        pipeline.close_trace(dictation, DictationOutcome::Pasted, &[]);
        // Then the failure is only logged
    }
}
