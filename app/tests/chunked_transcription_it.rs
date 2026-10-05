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

const CONDITIONS: [Condition; 5] = [
    Condition::Normal,
    Condition::Slow,
    Condition::Fast,
    Condition::SentencePauses,
    Condition::WordPauses,
];
const LEAD_INS_SECS: [f32; 3] = [0.0, 0.7, 1.4];
const MAX_LOST_BY_MERGE_SHARE: f64 = 0.02;
const MAX_DROPPED_PHRASE_WORDS: usize = 5;

#[test]
fn should_merge_chunked_speech_without_repeating_or_dropping_a_phrase() {
    // Given
    let Ok(base_url) = std::env::var("VOICEPUNCHER_TRANSCRIPTION_URL") else {
        eprintln!("GATED: VOICEPUNCHER_TRANSCRIPTION_URL not set, skipping");
        return;
    };
    let cfg = TranscriptionCfg {
        base_url,
        ..TranscriptionCfg::default()
    };
    let chunking = ChunkingCfg {
        chunk_threshold_secs: cfg.chunk_threshold_secs,
        chunk_overlap_secs: cfg.chunk_overlap_secs,
        vad_lookback_secs: cfg.vad_lookback_secs,
    };
    let mut transcriber = OpenAiTranscriber::connect(cfg).unwrap();
    let mut failures = Vec::new();
    let mut cases = 0;
    for lang in Lang::ALL {
        for condition in CONDITIONS {
            for lead_in in LEAD_INS_SECS {
                cases += 1;
                let speech = AudioBuffer16k::new(synthesize(lang, condition, lead_in).unwrap());
                // When
                let texts: Vec<String> = split_into_chunks(&speech, chunking)
                    .iter()
                    .filter_map(|chunk| transcriber.transcribe(chunk).unwrap())
                    .map(|raw| raw.as_str().to_owned())
                    .collect();
                let merged = merge_chunk_texts(texts.clone());
                // Then
                let repeated = repeated_phrases(lang.text(), &merged);
                let missing = missing_word_share(lang.text(), &merged);
                let joined = texts.join(" ");
                let lost_by_merge = missing - missing_word_share(lang.text(), &joined);
                let dropped = dropped_phrase_len(lang.text(), &joined);
                let case = format!("{lang:?} {condition:?} lead-in {lead_in} s");
                eprintln!(
                    "  {case:<36} repeated={repeated:?} missing={missing:.3} lost_by_merge={lost_by_merge:.3} dropped_phrase={dropped}"
                );
                if !repeated.is_empty()
                    || lost_by_merge > MAX_LOST_BY_MERGE_SHARE
                    || dropped > MAX_DROPPED_PHRASE_WORDS
                {
                    failures.push(format!(
                        "{case}: repeated {repeated:?}, merge dropped {:.1} % of the recognized words, chunks dropped a phrase of {dropped} words\n    merged: {merged}\n    chunks:\n      {}",
                        lost_by_merge * 100.0,
                        texts.join("\n      ")
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {cases} chunked transcriptions failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
