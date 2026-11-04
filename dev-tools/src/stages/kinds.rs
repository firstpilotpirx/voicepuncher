//! The kinds of stages: the model stages, which run under the e2e lock, among them the benches (`crate::benches`).

pub use crate::benches::is_bench;

/// Whether a stage is a model stage: it and every stage after it run under the e2e lock.
pub fn is_model(stage: &str) -> bool {
    stage.starts_with("e2e") || stage == "it-real" || is_bench(stage)
}
