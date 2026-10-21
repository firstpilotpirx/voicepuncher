use crate::core::domain::{Permission, PttSignal, SpottingEvent};

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
    /// The runtime ends this pipeline because `Permission` was revoked: as a shutdown, and
    /// a recording in progress ends with a failure notice naming the permission.
    PermissionRevoked(Permission),
}
