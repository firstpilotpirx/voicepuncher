//! Saved settings reach the running app through its runtime: the settings window cannot
//! name the runtime itself, so the menu bar adapter, which owns it, applies them.

use crate::adapters::primary::settings::macos::session::{
    ApplyStatus, ConfigApplier, StatusListener,
};
use crate::config::Config;
use crate::runtime::{ReloadReport, Reloader};

/// Applies saved settings by reloading the runtime.
pub struct RuntimeApplier {
    reloader: Reloader,
}

impl RuntimeApplier {
    pub fn new(reloader: Reloader) -> Self {
        Self { reloader }
    }
}

impl ConfigApplier for RuntimeApplier {
    fn apply(&self, config: Config, mut listener: StatusListener) {
        let reported = self
            .reloader
            .reload_reporting(config, Box::new(move |report| listener(status_of(report))));
        if let Err(e) = reported {
            // a configuration that does not validate was reported already
            log::warn!("settings: not applied: {e:#}");
        }
    }
}

fn status_of(report: ReloadReport) -> ApplyStatus {
    match report {
        ReloadReport::WaitsForDictation => ApplyStatus::WaitsForDictation,
        ReloadReport::Applied => ApplyStatus::Applied,
        ReloadReport::Rejected(reason) => ApplyStatus::Rejected(reason),
    }
}
