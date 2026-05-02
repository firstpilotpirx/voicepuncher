use std::path::Path;

use anyhow::{anyhow, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::core::application::ports::transcriber::Transcriber;
use crate::core::domain::{AudioBuffer16k, RawTranscript};

pub struct WhisperCppTranscriber {
    ctx: WhisperContext,
    language: String,
    threads: i32,
}

impl WhisperCppTranscriber {
    pub fn load(model_path: &Path, language: &str, threads: usize) -> Result<Self> {
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
        let resolved_threads = if threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            threads
        };
        Ok(Self {
            ctx,
            language: language.to_owned(),
            threads: resolved_threads as i32,
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

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(&self.language));
        params.set_n_threads(self.threads);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = self.ctx.create_state()?;
        state.full(params, &audio_f32)?;

        let mut result = String::new();
        for segment in state.as_iter() {
            result.push_str(segment.to_str()?);
        }

        RawTranscript::new(result).map_err(|e| anyhow!(e))
    }
}
