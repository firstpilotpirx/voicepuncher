use std::num::NonZeroU32;
use std::path::Path;

use anyhow::{anyhow, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use crate::core::application::text_refiner::TextRefiner;
use crate::core::domain::{CleanedText, RawTranscript};

pub struct LlamaCppTextRefiner {
    model: LlamaModel,
    system_prompt: String,
    max_tokens: i32,
    temperature: f32,
    context_size: u32,
    _backend: LlamaBackend,
}

impl LlamaCppTextRefiner {
    pub fn load(
        model_path: &Path,
        system_prompt: String,
        max_tokens: i32,
        temperature: f32,
        context_size: u32,
    ) -> Result<Self> {
        if !model_path.exists() {
            return Err(anyhow!("llama model not found: {}", model_path.display()));
        }

        let backend =
            LlamaBackend::init().map_err(|e| anyhow!("llama backend init failed: {e}"))?;
        let model = LlamaModel::load_from_file(&backend, model_path, &LlamaModelParams::default())
            .map_err(|e| anyhow!("failed to load llama model: {e}"))?;

        Ok(Self {
            model,
            system_prompt,
            max_tokens,
            temperature,
            context_size,
            _backend: backend,
        })
    }
}

impl TextRefiner for LlamaCppTextRefiner {
    fn refine(&mut self, raw: &RawTranscript) -> Result<CleanedText> {
        let prompt = format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            self.system_prompt,
            raw.as_str()
        );

        let ctx_params =
            LlamaContextParams::default().with_n_ctx(NonZeroU32::new(self.context_size));
        let mut ctx = self
            .model
            .new_context(&self._backend, ctx_params)
            .map_err(|e| anyhow!("failed to create llama context: {e}"))?;

        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| anyhow!("tokenization failed: {e}"))?;

        let mut batch = LlamaBatch::new(tokens.len(), 1);
        batch
            .add_sequence(&tokens, 0, false)
            .map_err(|e| anyhow!("batch add failed: {e}"))?;
        ctx.decode(&mut batch)
            .map_err(|e| anyhow!("decode failed: {e}"))?;

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(self.temperature),
            LlamaSampler::dist(42),
        ]);

        let mut output = String::new();
        let mut pos = tokens.len() as i32;
        let mut finished = false;

        for _ in 0..self.max_tokens {
            let token = sampler.sample(&ctx, -1);

            if self.model.is_eog_token(token) {
                finished = true;
                break;
            }

            let bytes = match self.model.token_to_piece_bytes(token, 8, false, None) {
                Err(llama_cpp_2::TokenToStringError::InsufficientBufferSpace(needed)) => self
                    .model
                    .token_to_piece_bytes(token, (-needed) as usize, false, None)
                    .map_err(|e| anyhow!("token decode failed: {e}"))?,
                Ok(b) => b,
                Err(e) => return Err(anyhow!("token decode failed: {e}")),
            };

            let piece = String::from_utf8_lossy(&bytes).into_owned();
            output.push_str(&piece);

            if output.contains("<|im_end|>") {
                output = output.replace("<|im_end|>", "");
                finished = true;
                break;
            }

            sampler.accept(token);

            batch.clear();
            batch
                .add(token, pos, &[0], true)
                .map_err(|e| anyhow!("batch add failed: {e}"))?;
            ctx.decode(&mut batch)
                .map_err(|e| anyhow!("decode failed: {e}"))?;

            pos += 1;
        }

        if !finished {
            log::warn!(
                "LLM hit max_tokens={} without reaching end-of-sequence — output may be truncated. \
                 Input tokens: {}, generated: {} chars",
                self.max_tokens,
                tokens.len(),
                output.len()
            );
        }

        CleanedText::new(output).map_err(|e| anyhow!(e))
    }
}
