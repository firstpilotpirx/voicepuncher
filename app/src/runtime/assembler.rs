//! How the runtime gets a pipeline for a configuration.

use std::sync::mpsc::Sender;

use anyhow::Result;

use crate::config::Config;
use crate::core::application::pipeline::Pipeline;
use crate::core::application::PipelineInput;

/// Builds a dictation pipeline from a configuration, probing every service it needs (and,
/// for the real one, the permissions, at the first assembly).
pub trait PipelineAssembler: Send + 'static {
    /// A pipeline for `config` that has not started; parts that report events, such as the
    /// command spotter, send them to `inputs`, the channel the pipeline will read.
    fn assemble(&mut self, config: &Config, inputs: Sender<PipelineInput>) -> Result<Pipeline>;
}
