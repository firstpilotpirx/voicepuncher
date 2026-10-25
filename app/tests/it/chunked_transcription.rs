// Seam sweep against a real speech recognition service, gated on
// VOICEPUNCHER_TRANSCRIPTION_URL: synthesized Russian and English speech is split with the
// chunk rule, every chunk goes through OpenAiTranscriber, and the merged text must repeat no
// phrase of the speech and drop at most one in fifty spoken words beyond those the chunks
// already missed (words lost inside a chunk are recognition, not the merge). The chunk
// transcripts themselves must drop no phrase of more than five spoken words, as the service
// does when it skips the speech after a pause. The lead-in moves the seams onto other words,
// because a duplicate depends on which word a seam cuts.

use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher::core::application::chunking::{merge_chunk_texts, split_into_chunks};
use voicepuncher::core::application::{ChunkingCfg, Transcriber};
use voicepuncher::core::domain::AudioBuffer16k;
use voicepuncher_e2e_test::speech::{
    dropped_phrase_len, missing_word_share, repeated_phrases, synthesize, Condition, Lang,
};
use voicepuncher_test_steps::required_env;

const CONDITIONS: [Condition; 5] = [
    Condition::Normal,
    Condition::Slow,
    Condition::Fast,
    Condition::SentencePauses,
    Condition::WordPauses,
];
const LEAD_INS_SECS: [f32; 3] = [0.0, 0.7, 1.4];

/// The lead-ins of the sweep: lead-in 0 alone in the seam sample (`VOICEPUNCHER_SEAM_SAMPLE=1`, the
/// `e2e-seams-sample` stage), all three in the whole matrix.
fn lead_ins() -> &'static [f32] {
    if std::env::var("VOICEPUNCHER_SEAM_SAMPLE").as_deref() == Ok("1") {
        &LEAD_INS_SECS[..1]
    } else {
        &LEAD_INS_SECS
    }
}
const MAX_LOST_BY_MERGE_SHARE: f64 = 0.02;
const MAX_DROPPED_PHRASE_WORDS: usize = 5;

mod needs_stt {
    use super::*;

    /// What went wrong when `lang` spoken under `condition` after `lead_in` was transcribed
    /// in chunks and merged; none when nothing did.
    fn merge_failure(
        transcriber: &mut OpenAiTranscriber,
        chunking: ChunkingCfg,
        (lang, condition, lead_in): (Lang, Condition, f32),
    ) -> Option<String> {
        let speech = AudioBuffer16k::new(synthesize(lang, condition, lead_in).unwrap());
        let texts: Vec<String> = split_into_chunks(&speech, chunking)
            .iter()
            .filter_map(|chunk| transcriber.transcribe(chunk).unwrap())
            .map(|raw| raw.as_str().to_owned())
            .collect();
        let merged = merge_chunk_texts(&texts);
        let repeated = repeated_phrases(lang.text(), &merged);
        let missing = missing_word_share(lang.text(), &merged);
        let joined = texts.join(" ");
        let lost_by_merge = missing - missing_word_share(lang.text(), &joined);
        let dropped = dropped_phrase_len(lang.text(), &joined);
        let case = format!("{lang:?} {condition:?} lead-in {lead_in} s");
        eprintln!(
            "  {case:<36} repeated={repeated:?} missing={missing:.3} lost_by_merge={lost_by_merge:.3} dropped_phrase={dropped}"
        );
        let failed = !repeated.is_empty()
            || lost_by_merge > MAX_LOST_BY_MERGE_SHARE
            || dropped > MAX_DROPPED_PHRASE_WORDS;
        failed.then(|| {
            format!(
                "{case}: repeated {repeated:?}, merge dropped {:.1} % of the recognized words, chunks dropped a phrase of {dropped} words\n    merged: {merged}\n    chunks:\n      {}",
                lost_by_merge * 100.0,
                texts.join("\n      ")
            )
        })
    }

    #[test]
    fn should_merge_chunked_speech_without_repeating_or_dropping_a_phrase() {
        // Given
        let base_url = required_env("VOICEPUNCHER_TRANSCRIPTION_URL");
        let cfg = TranscriptionCfg { base_url, ..TranscriptionCfg::default() };
        let chunking = ChunkingCfg {
            chunk_threshold_secs: cfg.chunk_threshold_secs,
            chunk_overlap_secs: cfg.chunk_overlap_secs,
            vad_lookback_secs: cfg.vad_lookback_secs,
        };
        let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
        let cases: Vec<(Lang, Condition, f32)> = Lang::ALL
            .into_iter()
            .flat_map(|lang| CONDITIONS.into_iter().map(move |c| (lang, c)))
            .flat_map(|(lang, c)| lead_ins().iter().map(move |&l| (lang, c, l)))
            .collect();
        // When
        let failures: Vec<String> = cases
            .iter()
            .filter_map(|&case| merge_failure(&mut transcriber, chunking, case))
            .collect();
        // Then
        assert!(
            failures.is_empty(),
            "{} of {} chunked transcriptions failed:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n")
        );
    }
}
