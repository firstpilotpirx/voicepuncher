use std::path::Path;

use anyhow::{anyhow, Result};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::config::WhisperCfg;
use crate::core::application::transcriber::Transcriber;
use crate::core::domain::{AudioBuffer16k, RawTranscript};

#[allow(dead_code)]
const FRAME_SAMPLES: usize = 320;

pub struct WhisperCppTranscriber {
    ctx: WhisperContext,
    cfg: WhisperCfg,
    threads: i32,
    state: Option<WhisperState>,
}

impl WhisperCppTranscriber {
    pub fn load(model_path: &Path, cfg: WhisperCfg) -> Result<Self> {
        if !model_path.exists() {
            return Err(anyhow!("whisper model not found: {}", model_path.display()));
        }
        let path_str = model_path.to_str().ok_or_else(|| {
            anyhow!(
                "whisper model path is not valid UTF-8: {}",
                model_path.display()
            )
        })?;
        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu(true);
        ctx_params.flash_attn(cfg.flash_attn);
        let ctx = WhisperContext::new_with_params(path_str, ctx_params)
            .map_err(|e| anyhow!("failed to load whisper model: {e}"))?;
        let resolved_threads = if cfg.threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            cfg.threads
        };
        Ok(Self {
            ctx,
            threads: resolved_threads as i32,
            cfg,
            state: None,
        })
    }
}

impl Transcriber for WhisperCppTranscriber {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<Option<RawTranscript>> {
        let t_start = std::time::Instant::now();
        let audio_secs = audio.samples().len() as f32 / AudioBuffer16k::SAMPLE_RATE_HZ as f32;

        let audio_f32: Vec<f32> = audio
            .samples()
            .iter()
            .map(|&s| s as f32 / 32_768.0)
            .collect();

        let strategy = if self.cfg.beam_size > 1 {
            SamplingStrategy::BeamSearch {
                beam_size: self.cfg.beam_size,
                patience: self.cfg.patience,
            }
        } else {
            SamplingStrategy::Greedy { best_of: 1 }
        };

        let mut params = FullParams::new(strategy);
        params.set_language(Some(&self.cfg.language));
        params.set_n_threads(self.threads);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        params.set_no_context(true);

        params.set_temperature(self.cfg.temperature);
        params.set_temperature_inc(self.cfg.temperature_inc);
        params.set_entropy_thold(self.cfg.entropy_thold);
        params.set_logprob_thold(self.cfg.logprob_thold);
        params.set_no_speech_thold(self.cfg.no_speech_thold);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_no_timestamps(true);
        params.set_single_segment(true);

        if !self.cfg.initial_prompt.is_empty() {
            params.set_initial_prompt(&self.cfg.initial_prompt);
        }

        let t_after_params = std::time::Instant::now();

        if self.state.is_none() {
            self.state = Some(self.ctx.create_state()?);
        }
        let state = self.state.as_mut().unwrap();

        let t_after_state = std::time::Instant::now();

        state.full(params, &audio_f32)?;

        let t_after_full = std::time::Instant::now();

        let mut result = String::new();
        for segment in state.as_iter() {
            result.push_str(segment.to_str()?);
        }

        let t_after_segments = std::time::Instant::now();

        let hallucinated = is_whisper_hallucination(&result);

        let t_after_hallu = std::time::Instant::now();

        let total_ms = (t_after_hallu - t_start).as_millis();
        if audio_secs >= 8.0 || total_ms > 600 {
            log::info!(
                "whisper transcribe: audio_secs={:.2} prep={}ms state={}ms full={}ms segments={}ms hallu_check={}ms total={}ms",
                audio_secs,
                (t_after_params - t_start).as_millis(),
                (t_after_state - t_after_params).as_millis(),
                (t_after_full - t_after_state).as_millis(),
                (t_after_segments - t_after_full).as_millis(),
                (t_after_hallu - t_after_segments).as_millis(),
                total_ms,
            );
        }

        if hallucinated {
            log::info!("dropping hallucinated transcript: {:?}", result.trim());
            return Ok(None);
        }

        RawTranscript::new(result).map(Some).map_err(|e| anyhow!(e))
    }
}

#[cfg(test)]
fn vad_silence_frames(audio: &[f32], rms_threshold: f32) -> Vec<usize> {
    let frame_count = audio.len().div_ceil(FRAME_SAMPLES);
    (0..frame_count)
        .filter(|&i| {
            let start = i * FRAME_SAMPLES;
            let end = (start + FRAME_SAMPLES).min(audio.len());
            let frame = &audio[start..end];
            let mean_sq = frame.iter().map(|&s| s * s).sum::<f32>() / frame.len() as f32;
            mean_sq.sqrt() < rms_threshold
        })
        .collect()
}

#[cfg(test)]
fn find_chunk_boundaries(
    total_samples: usize,
    chunk_samples: usize,
    overlap_samples: usize,
    lookback_samples: usize,
    silence_frames: &[usize],
) -> Vec<(usize, usize)> {
    let chunk_count = total_samples.div_ceil(chunk_samples);
    let mut cuts: Vec<usize> = Vec::with_capacity(chunk_count);

    for k in 1..chunk_count {
        let hard_boundary = k * chunk_samples;
        let lookback_start = hard_boundary.saturating_sub(lookback_samples);
        let lookback_start_frame = lookback_start / FRAME_SAMPLES;
        let hard_boundary_frame = hard_boundary / FRAME_SAMPLES;

        let cut_sample = silence_frames
            .iter()
            .rev()
            .find(|&&frame_idx| {
                frame_idx >= lookback_start_frame && frame_idx < hard_boundary_frame
            })
            .map(|&frame_idx| frame_idx * FRAME_SAMPLES)
            .unwrap_or(hard_boundary);

        cuts.push(cut_sample);
    }
    cuts.push(total_samples);

    let mut boundaries = Vec::with_capacity(chunk_count);
    for (idx, &end) in cuts.iter().enumerate() {
        let start = if idx == 0 {
            0
        } else {
            cuts[idx - 1].saturating_sub(overlap_samples)
        };
        boundaries.push((start, end));
    }

    boundaries
}

#[cfg(test)]
#[allow(dead_code)]
fn transcribe_chunk_with_state(
    state: &mut WhisperState,
    audio_f32: &[f32],
    cfg: &WhisperCfg,
    threads: i32,
) -> Result<String> {
    let strategy = if cfg.beam_size > 1 {
        SamplingStrategy::BeamSearch {
            beam_size: cfg.beam_size,
            patience: cfg.patience,
        }
    } else {
        SamplingStrategy::Greedy { best_of: 1 }
    };

    let mut params = FullParams::new(strategy);
    params.set_language(Some(&cfg.language));
    params.set_n_threads(threads);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_no_context(true);
    params.set_temperature(cfg.temperature);
    params.set_temperature_inc(cfg.temperature_inc);
    params.set_entropy_thold(cfg.entropy_thold);
    params.set_logprob_thold(cfg.logprob_thold);
    params.set_no_speech_thold(cfg.no_speech_thold);
    params.set_suppress_blank(true);
    params.set_suppress_nst(true);
    params.set_no_timestamps(true);
    params.set_single_segment(true);

    if !cfg.initial_prompt.is_empty() {
        params.set_initial_prompt(&cfg.initial_prompt);
    }

    state.full(params, audio_f32)?;

    let mut text = String::new();
    for segment in state.as_iter() {
        text.push_str(segment.to_str()?);
    }

    if is_whisper_hallucination(&text) {
        log::info!("dropping hallucinated chunk transcript: {:?}", text.trim());
        return Ok(String::new());
    }

    Ok(text)
}

fn is_whisper_hallucination(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }
    const PATTERNS: &[&str] = &[
        "субтитры создавал",
        "продолжение следует",
        "спасибо за просмотр",
        "подписывайтесь на канал",
        "ставьте лайк",
        "thank you for watching",
        "thanks for watching",
        "please subscribe",
        "like and subscribe",
        "subtitles by",
        "subtitled by",
        "captions by",
        "[music]",
        "[applause]",
        "[blank_audio]",
        "(music)",
        "(applause)",
    ];
    PATTERNS.iter().any(|p| lower.contains(p)) || has_repetition_loop(&lower)
}

fn has_repetition_loop(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 10 {
        return false;
    }
    let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for &w in &words {
        *counts.entry(w).or_insert(0) += 1;
    }
    let max_freq = counts.values().max().copied().unwrap_or(0);
    max_freq * 10 > words.len() * 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::chunking::merge_chunk_texts;

    #[test]
    fn should_detect_russian_subtitle_hallucination() {
        assert!(is_whisper_hallucination(" Субтитры создавал DimaTorzok"));
    }

    #[test]
    fn should_detect_continuation_hallucination() {
        assert!(is_whisper_hallucination("Продолжение следует..."));
    }

    #[test]
    fn should_detect_english_subscribe_hallucination() {
        assert!(is_whisper_hallucination("Thank you for watching!"));
    }

    #[test]
    fn should_detect_blank_audio_token() {
        assert!(is_whisper_hallucination("[BLANK_AUDIO]"));
    }

    #[test]
    fn should_not_filter_real_speech() {
        assert!(!is_whisper_hallucination("Hello, how are you doing?"));
        assert!(!is_whisper_hallucination("Привет, как дела?"));
    }

    #[test]
    fn should_not_filter_empty_string() {
        assert!(!is_whisper_hallucination(""));
        assert!(!is_whisper_hallucination("   "));
    }

    #[test]
    fn should_detect_repeated_phrase_loop() {
        let repeated = "и сайта, ".repeat(20);
        assert!(is_whisper_hallucination(&repeated));
    }

    #[test]
    fn should_not_filter_speech_with_moderate_repetition() {
        assert!(!is_whisper_hallucination(
            "и потом я пошёл и потом я взял и потом позвонил другу"
        ));
    }

    #[test]
    fn should_return_all_frame_indices_for_silence() {
        // Given
        let silence = vec![0.0f32; FRAME_SAMPLES * 5];
        // When
        let frames = vad_silence_frames(&silence, 0.01);
        // Then
        assert_eq!(frames, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn should_return_empty_vec_for_loud_audio() {
        // Given
        let loud = vec![0.5f32; FRAME_SAMPLES * 3];
        // When
        let frames = vad_silence_frames(&loud, 0.01);
        // Then
        assert!(frames.is_empty());
    }

    #[test]
    fn should_find_boundaries_for_three_chunks() {
        // Given — 3 × 1600 samples, overlap=320, no silence (hard cuts only)
        let total = 4800usize;
        let chunk_samples = 1600usize;
        let overlap = 320usize;
        // When
        let boundaries = find_chunk_boundaries(total, chunk_samples, overlap, 0, &[]);
        // Then — 3 chunks, contiguous with overlap prefix on chunks 2+, last ends at total
        assert_eq!(boundaries.len(), 3);
        assert_eq!(boundaries[0], (0, 1600));
        assert_eq!(boundaries[1], (1280, 3200));
        assert_eq!(boundaries[2], (2880, 4800));
        assert_eq!(boundaries.last().unwrap().1, total);
    }

    #[test]
    fn should_merge_three_chunks_dropping_overlap_words() {
        // Given — chunk 1: 4 words, chunks 2+3: 4 words each with 1-word overlap prefix
        // overlap_word_fraction = 0.25 → ceil(4 * 0.25) = 1 word dropped from chunks 2+3
        let texts = vec![
            "alpha beta gamma delta".to_string(),
            "delta epsilon zeta eta".to_string(),
            "eta theta iota kappa".to_string(),
        ];
        // When
        let merged = merge_chunk_texts(texts, 0.25);
        // Then — chunk1 kept whole, 1 word dropped from each of chunk2/3
        let words: Vec<&str> = merged.split_whitespace().collect();
        assert_eq!(
            words,
            vec![
                "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota",
                "kappa"
            ]
        );
    }

    #[test]
    fn should_detect_silence_frames_below_threshold() {
        // Given
        let silence = vec![0.0f32; FRAME_SAMPLES * 3];
        // When
        let frames = vad_silence_frames(&silence, 0.01);
        // Then
        assert_eq!(frames, vec![0, 1, 2]);
    }

    #[test]
    fn should_not_include_loud_frames_in_silence() {
        // Given
        let mut audio = vec![0.0f32; FRAME_SAMPLES * 2];
        for s in &mut audio[FRAME_SAMPLES..] {
            *s = 0.5;
        }
        // When
        let frames = vad_silence_frames(&audio, 0.01);
        // Then
        assert_eq!(frames, vec![0]);
    }

    #[test]
    fn should_produce_single_chunk_for_audio_shorter_than_chunk_size() {
        // Given
        let total = 1000;
        let chunk_samples = 2000;
        // When
        let boundaries = find_chunk_boundaries(total, chunk_samples, 0, 0, &[]);
        // Then
        assert_eq!(boundaries, vec![(0, 1000)]);
    }

    #[test]
    fn should_split_audio_into_two_chunks_at_hard_boundary() {
        // Given
        let total = 3200;
        let chunk_samples = 1600;
        // When
        let boundaries = find_chunk_boundaries(total, chunk_samples, 0, 0, &[]);
        // Then
        assert_eq!(boundaries, vec![(0, 1600), (1600, 3200)]);
    }

    #[test]
    fn should_apply_overlap_to_second_chunk_start() {
        // Given
        let total = 3200;
        let chunk_samples = 1600;
        let overlap = 320;
        // When
        let boundaries = find_chunk_boundaries(total, chunk_samples, overlap, 0, &[]);
        // Then
        assert_eq!(boundaries[1].0, 1280);
    }

    #[test]
    fn should_snap_cut_to_silence_within_lookback_window() {
        // Given
        let total = 3200;
        let chunk_samples = 1600;
        let silence_frame = 4;
        let silence_frames = vec![silence_frame];
        // When
        let boundaries = find_chunk_boundaries(total, chunk_samples, 0, 640, &silence_frames);
        // Then
        let cut = silence_frame * FRAME_SAMPLES;
        assert_eq!(boundaries[0].1, cut);
        assert_eq!(boundaries[1].0, cut);
    }

    #[test]
    fn should_always_end_last_chunk_at_total_samples() {
        // Given
        let total = 5000;
        let chunk_samples = 1600;
        // When
        let boundaries = find_chunk_boundaries(total, chunk_samples, 0, 0, &[]);
        // Then
        assert_eq!(boundaries.last().unwrap().1, total);
    }

    #[test]
    fn should_merge_chunk_texts_keeping_all_words_from_first_chunk() {
        // Given
        let texts = vec!["hello world".to_string(), "world foo bar".to_string()];
        // When
        let merged = merge_chunk_texts(texts, 0.5);
        // Then
        assert_eq!(merged, "hello world bar");
    }

    #[test]
    fn should_merge_chunk_texts_skipping_empty_hallucination_chunks() {
        // Given
        let texts = vec![
            "one two three".to_string(),
            String::new(),
            "three four five".to_string(),
        ];
        // When
        let merged = merge_chunk_texts(texts, 0.33);
        // Then
        assert!(merged.contains("one"));
        assert!(merged.contains("four") || merged.contains("five"));
    }

    #[test]
    fn should_merge_single_chunk_unchanged() {
        // Given
        let texts = vec!["hello world".to_string()];
        // When
        let merged = merge_chunk_texts(texts, 0.5);
        // Then
        assert_eq!(merged, "hello world");
    }

    #[test]
    fn should_merge_all_empty_chunks_to_empty_string() {
        // Given
        let texts = vec![String::new(), String::new()];
        // When
        let merged = merge_chunk_texts(texts, 0.5);
        // Then
        assert_eq!(merged, "");
    }
}
