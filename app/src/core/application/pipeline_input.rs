use crate::core::domain::{PttSignal, SpottingEvent};

/// Everything the pipeline reacts to, in one ordered stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineInput {
    Hotkey(PttSignal),
    Spotting(SpottingEvent),
    /// The command spotting service stopped answering.
    SpottingLost,
    /// The command spotting service has been away long enough that the replay to it on its
    /// return no longer hides the outage.
    SpottingStillAway,
    /// The command spotting service answers again after it was lost.
    SpottingBack,
    /// The runtime ends this pipeline: a recording in progress is cancelled, listening stops.
    Shutdown,
}
