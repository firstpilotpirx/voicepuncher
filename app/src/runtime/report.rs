//! What became of one reload, for whoever asked for it.

/// The outcome of a reload, told to the one who asked for it as it happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadReport {
    /// A dictation is in progress; the new settings apply once it has ended.
    WaitsForDictation,
    /// The pipeline runs with the new settings.
    Applied,
    /// The new settings were not applied, for this reason; the pipeline keeps its settings.
    Rejected(String),
}

/// Told every [`ReloadReport`] of one reload, from the runtime thread.
pub type ReloadListener = Box<dyn FnMut(ReloadReport) + Send>;
