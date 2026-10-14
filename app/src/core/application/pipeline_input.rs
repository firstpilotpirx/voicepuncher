use crate::core::domain::{PttSignal, SpottingEvent};

/// Everything the pipeline reacts to, in one ordered stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineInput {
    Hotkey(PttSignal),
    Spotting(SpottingEvent),
    /// The command spotting service stopped answering.
    SpottingLost,
    /// The command spotting service answers again after it was lost.
    SpottingBack,
}
