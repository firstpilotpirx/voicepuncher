//! The streaming recognizer of the service: its models, loaded once and shared by every
//! connection, and each connection's session.

use std::path::PathBuf;

use anyhow::Result;

use crate::history::{CommandHistory, HistoryCfg};

use super::ports::StreamingModel;
use super::session::{Session, SessionCfg};
use super::sherpa::{SherpaModel, SileroGate};

pub struct StreamingCfg {
    /// The English streaming model's directory.
    pub en_model_dir: PathBuf,
    /// The Russian one, whose Cyrillic command words are respelled.
    pub ru_model_dir: PathBuf,
    pub vad_model: PathBuf,
    pub threads: i32,
    pub min_silence_secs: f32,
    pub command_history: Option<HistoryCfg>,
}

pub struct StreamingEngine {
    models: Vec<SherpaModel>,
    vad_model: PathBuf,
    min_silence_secs: f32,
    history: Option<CommandHistory>,
}

impl StreamingEngine {
    pub fn load(cfg: &StreamingCfg) -> Result<Self> {
        let models = vec![
            SherpaModel::load("en", &cfg.en_model_dir, false, cfg.threads)?,
            SherpaModel::load("ru", &cfg.ru_model_dir, true, cfg.threads)?,
        ];
        // WHY: the gate loads per connection; a wrong path fails at start, not at connect.
        SileroGate::load(&cfg.vad_model)?;
        Ok(Self {
            models,
            vad_model: cfg.vad_model.clone(),
            min_silence_secs: cfg.min_silence_secs,
            history: cfg
                .command_history
                .clone()
                .map(CommandHistory::start)
                .transpose()?,
        })
    }

    /// The session of connection `conn`.
    pub fn session_for(&self, conn: u64) -> Result<Session<'_>> {
        let models = self
            .models
            .iter()
            .map(|m| m as &dyn StreamingModel)
            .collect();
        let gate = Box::new(SileroGate::load(&self.vad_model)?);
        let cfg = SessionCfg {
            conn,
            min_silence_secs: self.min_silence_secs,
            history: self.history.as_ref(),
        };
        Ok(Session::new(models, gate, &cfg))
    }

    /// Waits until every command history entry closed so far is written or given up.
    pub fn flush_history(&self) {
        if let Some(history) = self.history.as_ref() {
            history.flush();
        }
    }
}
