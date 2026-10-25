//! The model files the runtime waits for before it starts any local service.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::adapters::secondary::modelfetch::fs::{FsDiskSpace, FsModelStore};
use crate::adapters::secondary::modelfetch::http::HttpModelFetcher;
use crate::config::Config;
use crate::core::application::{Clock, DownloadEvent, ModelDownload, ModelStore};
use crate::core::domain::model_catalog::{self, ModelNeeds};

/// Replaces every model address with `<mirror>/<name>`, for tests.
pub const MIRROR_VARIABLE: &str = "VOICEPUNCHER_MODEL_MIRROR";

/// Which model files a configuration needs that are not present.
pub trait ModelsGate: Send {
    /// The folder downloads go to for `config`.
    fn folder(&self, config: &Config) -> PathBuf;
    /// The needed files that are missing; empty when every one is present.
    fn missing(&self, config: &Config) -> Vec<&'static str>;
}

/// The parts of the app `config` turns on, for the model catalog.
pub fn needs_of(config: &Config) -> ModelNeeds {
    ModelNeeds { voice_control: config.voice_control.enabled }
}

/// The models folder on this machine, the older folders and the variables naming a model.
pub struct CatalogModels {
    home: PathBuf,
    env: HashMap<String, String>,
}

impl CatalogModels {
    pub fn new(home: &Path, env: HashMap<String, String>) -> Self {
        Self { home: home.to_path_buf(), env }
    }

    /// This process's home and environment.
    pub fn current() -> Self {
        Self::new(
            &dirs::home_dir().unwrap_or_default(),
            std::env::vars().collect(),
        )
    }

    /// The store of `config`'s models folder.
    pub fn store(&self, config: &Config) -> FsModelStore {
        FsModelStore::new(self.folder(config), &self.home, self.env.clone())
    }
}

impl ModelsGate for CatalogModels {
    fn folder(&self, config: &Config) -> PathBuf {
        config.services.resolved_models_dir(&self.home)
    }

    fn missing(&self, config: &Config) -> Vec<&'static str> {
        let store = self.store(config);
        model_catalog::needed(needs_of(config))
            .into_iter()
            .filter(|file| !store.present(file))
            .map(|file| file.name)
            .collect()
    }
}

impl CatalogModels {
    /// The download of the files `needs` takes into `folder`, from the pinned addresses or
    /// the mirror the environment names.
    pub fn download(
        &self,
        folder: PathBuf,
        needs: ModelNeeds,
        clock: Arc<dyn Clock>,
    ) -> ModelDownload {
        ModelDownload::new(
            model_catalog::needed(needs),
            Box::new(FsModelStore::new(
                folder.clone(),
                &self.home,
                self.env.clone(),
            )),
            Box::new(HttpModelFetcher::new()),
            Box::new(FsDiskSpace::new(folder)),
            clock,
        )
        .with_mirror(self.env.get(MIRROR_VARIABLE).cloned())
    }
}

/// Downloads every missing file of `needs` into `folder` without a window, one line per
/// file to `out`; the process's exit code: 0 when every file is present at the end, else 1.
pub fn download_headless(
    models: &CatalogModels,
    folder: &Path,
    needs: ModelNeeds,
    clock: Arc<dyn Clock>,
    out: &mut dyn Write,
) -> i32 {
    let download = models.download(folder.to_path_buf(), needs, clock);
    let report = download.check();
    let _ = writeln!(
        out,
        "models folder {}: {} of {} files present",
        folder.display(),
        report.present_count(),
        report.rows.len()
    );
    let mut last_percent = None;
    let summary = download.download_missing(
        &mut |event: &DownloadEvent| {
            if let Some(line) = headless_line(event, &mut last_percent) {
                let _ = writeln!(out, "{line}");
            }
        },
        &AtomicBool::new(false),
    );
    i32::from(!summary.all_present)
}

/// The line an event prints; progress only every 10 %.
fn headless_line(event: &DownloadEvent, last_percent: &mut Option<u64>) -> Option<String> {
    match event {
        DownloadEvent::NotEnoughSpace { needed, free } => Some(format!(
            "not enough space: {needed} bytes needed, {free} bytes free"
        )),
        DownloadEvent::Started(name) => {
            *last_percent = None;
            Some(format!("{name}: downloading"))
        }
        DownloadEvent::Progress { name, received, total, .. } => {
            let percent = (received * 100).checked_div(*total).unwrap_or(100) / 10 * 10;
            (*last_percent != Some(percent)).then(|| {
                *last_percent = Some(percent);
                format!("{name}: {percent} %")
            })
        }
        DownloadEvent::AllPresent => Some("every model is present".into()),
        file_event => file_line(file_event),
    }
}

/// The line of an event about one file after its download started.
fn file_line(event: &DownloadEvent) -> Option<String> {
    match event {
        DownloadEvent::Verifying(name) => Some(format!("{name}: verifying the part")),
        DownloadEvent::Retrying { name, attempt, pause } => Some(format!(
            "{name}: no connection, try {} in {} s",
            attempt + 1,
            pause.as_secs()
        )),
        DownloadEvent::MismatchRetrying(name) => {
            Some(format!("{name}: checksum mismatch, downloading again"))
        }
        DownloadEvent::Done(name) => Some(format!("{name}: done")),
        DownloadEvent::Failed { name, failure } => Some(format!("{name}: failed: {failure}")),
        DownloadEvent::Cancelled(name) => Some(format!("{name}: cancelled")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_the_dictation_models_missing_from_an_empty_home() {
        // Given
        let home = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.voice_control.enabled = false;
        // When
        let missing = CatalogModels::new(home.path(), HashMap::new()).missing(&config);
        // Then
        assert_eq!(
            missing,
            vec!["ggml-large-v3-turbo-q5_0.bin", "ggml-silero-v5.1.2.bin"]
        );
    }

    #[test]
    fn should_take_the_models_named_by_their_variables() {
        // Given
        let home = tempfile::tempdir().unwrap();
        let named = home.path().join("named.bin");
        std::fs::write(&named, b"x").unwrap();
        let env: HashMap<String, String> = ["VOICEPUNCHER_WHISPER_MODEL", "VOICEPUNCHER_VAD_MODEL"]
            .into_iter()
            .map(|var| (var.to_string(), named.display().to_string()))
            .collect();
        let mut config = Config::default();
        config.voice_control.enabled = false;
        // When
        let missing = CatalogModels::new(home.path(), env).missing(&config);
        // Then
        assert!(missing.is_empty(), "{missing:?}");
    }
}
