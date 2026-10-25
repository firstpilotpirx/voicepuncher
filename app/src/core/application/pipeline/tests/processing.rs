//! A stopped dictation on its way to the paste, and file mode.

use super::*;
use crate::core::application::ChunkingCfg;
use crate::core::domain::{FailingPart, FailureNotice, RawTranscript};

/// Dictates "hello" through a pipeline whose doubles the test chose; what was pasted.
fn dictate(pipeline: PipelineUnderTest) -> Vec<String> {
    let (paster, texts) = noting_paster();
    let mut pipeline = pipeline.paster(paster).build();
    press(&mut pipeline);
    press(&mut pipeline);
    seen(&texts)
}

#[test]
fn should_paste_the_transcript_without_its_filler_sounds_after_the_stop() {
    assert_eq!(
        dictate(PipelineUnderTest::new().transcriber(transcribing("Эм, hello, э-э, world."))),
        ["hello, world."]
    );
}

#[test]
fn should_paste_nothing_and_announce_the_dictation_lost_when_transcription_fails() {
    // Given
    let (overlay, shown) = noting_overlay();
    let under_test = PipelineUnderTest::new()
        .transcriber(transcription_down())
        .overlay(overlay);
    // When
    let texts = dictate(under_test);
    // Then
    let notice = FailureNotice::DictationLost(FailingPart::SpeechRecognition);
    assert!(texts.is_empty());
    assert_eq!(seen(&shown).last(), Some(&format!("{notice:?}")));
}

#[test]
fn should_announce_the_text_not_pasted_when_the_paste_fails() {
    // Given
    let (overlay, shown) = noting_overlay();
    let mut pipeline = PipelineUnderTest::new()
        .paster(paste_down())
        .overlay(overlay)
        .build();
    press(&mut pipeline);
    // When
    press(&mut pipeline);
    // Then
    let notice = FailureNotice::NotPasted(None);
    assert_eq!(seen(&shown).last(), Some(&format!("{notice:?}")));
}

/// The stage names of a `stop→paste` line, each checked to carry a duration in ms.
fn stage_names(line: &str) -> Vec<&str> {
    line.strip_prefix("stop→paste: ")
        .unwrap()
        .split(' ')
        .map(|stage| {
            let (name, ms) = stage.split_once('=').unwrap();
            ms.strip_suffix("ms").unwrap().parse::<u64>().unwrap();
            name
        })
        .collect()
}

#[test]
fn should_log_the_stop_to_paste_line_with_every_stage_and_the_total() {
    // Given
    let log = captured_log();
    let mut pipeline = PipelineUnderTest::new().build();
    press(&mut pipeline);
    // When
    press(&mut pipeline);
    // Then
    let lines = log.starting_with("stop→paste: ");
    let line = lines.last().expect("a stop→paste line");
    assert_eq!(stage_names(line), ["stop", "transcribe", "paste", "total"]);
}

#[test]
fn should_run_once_and_return_the_pasted_text() {
    // Given
    let mut pipeline = PipelineUnderTest::new()
        .recorder(idle_recorder())
        .transcriber(transcribing("um Hello."))
        .build();
    // When
    let pasted = pipeline.run_once(one_second()).unwrap();
    // Then
    assert_eq!(
        pasted.map(|text| text.as_str().to_owned()).as_deref(),
        Some("Hello.")
    );
}

#[test]
fn should_run_once_and_fail_when_transcription_fails() {
    // Given
    let mut pipeline = PipelineUnderTest::new()
        .recorder(idle_recorder())
        .transcriber(transcription_down())
        .build();
    // When
    let result = pipeline.run_once(one_second());
    // Then
    assert!(result.is_err());
}

#[test]
fn should_run_once_in_chunks_when_the_recording_is_longer_than_the_threshold() {
    // Given — 70 s of sound and a 30 s chunk threshold: chunks end at 30 s, 60 s, 70 s
    let mut texts = ["one two three", "three four five", "five six"].into_iter();
    let mut transcriber = MockTranscriber::new();
    transcriber
        .expect_transcribe()
        .times(3)
        .returning(move |_| Ok(Some(RawTranscript::new(texts.next().unwrap().to_owned())?)));
    let (paster, texts) = noting_paster();
    let chunking = ChunkingCfg { chunk_threshold_secs: 30, ..ChunkingCfg::default() };
    let mut pipeline = PipelineUnderTest::new()
        .recorder(idle_recorder())
        .transcriber(transcriber)
        .paster(paster)
        .cfg(PipelineConfig { chunking, ..PipelineConfig::default() })
        .build();
    // When
    pipeline
        .run_once(AudioBuffer16k::new(vec![10_000i16; 70 * 16_000]))
        .unwrap();
    // Then — three transcriptions and one merged paste
    assert_eq!(seen(&texts), ["one two three four five six"]);
}
