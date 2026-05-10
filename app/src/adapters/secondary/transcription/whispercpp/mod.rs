use std::path::Path;

use anyhow::{anyhow, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::config::WhisperCfg;
use crate::core::application::transcriber::Transcriber;
use crate::core::domain::{AudioBuffer16k, RawTranscript};

pub struct WhisperCppTranscriber {
    ctx: WhisperContext,
    cfg: WhisperCfg,
    threads: i32,
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
        let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
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
        })
    }
}

impl Transcriber for WhisperCppTranscriber {
    fn transcribe(&mut self, audio: &AudioBuffer16k) -> Result<RawTranscript> {
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

        // Prevent error propagation across segments in long recordings.
        params.set_no_context(true);

        params.set_temperature(self.cfg.temperature);
        params.set_temperature_inc(self.cfg.temperature_inc);
        params.set_entropy_thold(self.cfg.entropy_thold);
        params.set_logprob_thold(self.cfg.logprob_thold);
        params.set_no_speech_thold(self.cfg.no_speech_thold);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);

        if !self.cfg.initial_prompt.is_empty() {
            params.set_initial_prompt(&self.cfg.initial_prompt);
        }

        let mut state = self.ctx.create_state()?;
        state.full(params, &audio_f32)?;

        let mut result = String::new();
        for segment in state.as_iter() {
            result.push_str(segment.to_str()?);
        }

        if is_whisper_hallucination(&result) {
            log::info!("dropping hallucinated transcript: {:?}", result.trim());
            result.clear();
        }

        RawTranscript::new(result).map_err(|e| anyhow!(e))
    }
}

fn is_whisper_hallucination(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }
    const PATTERNS: &[&str] = &[
        // Russian training-data artifacts
        "субтитры создавал",
        "продолжение следует",
        "спасибо за просмотр",
        "подписывайтесь на канал",
        "ставьте лайк",
        // English training-data artifacts
        "thank you for watching",
        "thanks for watching",
        "please subscribe",
        "like and subscribe",
        "subtitles by",
        "subtitled by",
        "captions by",
        // Special tokens that leak through as text
        "[music]",
        "[applause]",
        "[blank_audio]",
        "(music)",
        "(applause)",
    ];
    PATTERNS.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_detect_russian_subtitle_hallucination() {
        // Given / When / Then
        assert!(is_whisper_hallucination(
            " Субтитры создавал DimaTorzok"
        ));
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
        assert!(!is_whisper_hallucination(
            "Привет, как дела?"
        ));
    }

    #[test]
    fn should_not_filter_empty_string() {
        assert!(!is_whisper_hallucination(""));
        assert!(!is_whisper_hallucination("   "));
    }
}
