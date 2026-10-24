/// What the pipeline is doing, as far as a restart must care: a dictation in progress is
/// never cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipelineActivity {
    #[default]
    Idle,
    Recording,
    /// Recognition, the focus wait and the paste.
    Processing,
}

/// Where the pipeline publishes its activity on every transition.
pub trait PipelineActivitySink: Send {
    fn publish(&self, activity: PipelineActivity);
}

/// The sink of a pipeline nobody restarts.
pub struct NoActivitySink;

impl PipelineActivitySink for NoActivitySink {
    fn publish(&self, _activity: PipelineActivity) {}
}
