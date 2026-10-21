//! The model files the runtime waits for: a models folder in a home of its own, filled with
//! files of the catalog size (sparse, so a multi-gigabyte model costs no disk), checked by the
//! app's own catalog gate.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Result};
use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::core::domain::model_catalog::{self, ModelFile, ModelKind};
use voicepuncher::runtime::{models::needs_of, CatalogModels, ModelsGate, RuntimeState};

use super::services::FakeServices;
use super::{run, Event, RuntimeRun, SETTLE};
use crate::IntegrationWorld;

pub(crate) const DICTATION_MODEL: &str = "ggml-large-v3-turbo-q5_0.bin";

/// A home with an empty models folder.
pub(crate) struct ModelsFolder {
    home: TempDir,
}

impl ModelsFolder {
    fn new() -> Self {
        Self {
            home: tempfile::tempdir().expect("a temporary home"),
        }
    }

    pub(crate) fn folder(&self) -> PathBuf {
        self.home.path().join("models")
    }

    /// The app's gate over this folder; no variable names a model.
    pub(crate) fn gate(&self) -> Box<dyn ModelsGate> {
        Box::new(CatalogModels::new(self.home.path(), HashMap::new()))
    }

    /// Puts `file` in the folder at its catalog size.
    pub(crate) fn place(&self, file: &ModelFile) {
        let folder = self.folder();
        std::fs::create_dir_all(&folder).expect("the models folder");
        match file.kind {
            ModelKind::File => std::fs::File::create(folder.join(file.name))
                .and_then(|placed| placed.set_len(file.size_bytes))
                .expect("a model file"),
            ModelKind::TarBz2 { folder: name } => {
                std::fs::create_dir_all(folder.join(name)).expect("a model folder")
            }
        }
    }
}

impl RuntimeRun {
    /// The runtime manages its services, which answer by themselves, and waits for the model
    /// files of its configuration; every needed file but `missing` is in the models folder.
    pub(crate) fn manage_with_models(&mut self, missing: &[&str]) {
        let models = ModelsFolder::new();
        self.config.voice_control.enabled = false;
        self.config.services.models_dir = models.folder();
        for file in model_catalog::needed(needs_of(&self.config)) {
            if !missing.contains(&file.name) {
                models.place(&file);
            }
        }
        self.services = Some(FakeServices::new(&self.events, true));
        self.models = Some(models);
    }

    /// Places every needed file that is still missing.
    pub(crate) fn place_missing_models(&self) -> Result<()> {
        let Some(models) = &self.models else {
            bail!("the runtime waits for no models");
        };
        for file in model_catalog::needed(needs_of(&self.config)) {
            models.place(&file);
        }
        Ok(())
    }

    pub(crate) fn config(&self) -> &voicepuncher::config::Config {
        &self.config
    }

    /// The home whose models folder the runtime checks.
    pub(crate) fn models_home(&self) -> Option<PathBuf> {
        self.models
            .as_ref()
            .map(|models| models.home.path().to_path_buf())
    }

    pub(crate) fn waits_for_models(&self) -> Result<()> {
        self.state_becomes(|s| matches!(s, RuntimeState::WaitingForModels { .. }))
    }

    pub(crate) fn ever_waited_for_models(&self) -> bool {
        self.count(|e| matches!(e, Event::State(RuntimeState::WaitingForModels { .. }))) > 0
    }

    pub(crate) fn any_service_required(&self) -> bool {
        self.services
            .as_ref()
            .is_some_and(|services| services.any_required())
    }

    /// Ready, with Starting services told on the way since the runtime last waited for models.
    pub(crate) fn starting_services_then_ready(&self) -> Result<()> {
        self.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
        let waited =
            self.position(|e| matches!(e, Event::State(RuntimeState::WaitingForModels { .. })))?;
        let states: Vec<RuntimeState> = self.events()[waited + 1..]
            .iter()
            .filter_map(|event| match event {
                Event::State(state) if *state != RuntimeState::Starting => Some(state.clone()),
                _ => None,
            })
            .filter(|state| !matches!(state, RuntimeState::WaitingForModels { .. }))
            .collect();
        match states.as_slice() {
            [RuntimeState::StartingServices { .. }, .., RuntimeState::Ready { .. }] => Ok(()),
            _ => bail!("expected Starting services and then Ready, got {states:?}"),
        }
    }
}

#[given("every permission is granted and the dictation model is missing")]
fn dictation_model_missing(world: &mut IntegrationWorld) {
    crate::steps::permissions_gate::all_granted(world);
    run(world).manage_with_models(&[DICTATION_MODEL]);
}

#[given("the runtime is Ready with every model present")]
fn ready_with_every_model(world: &mut IntegrationWorld) -> Result<()> {
    crate::steps::permissions_gate::all_granted(world);
    let run = run(world);
    run.manage_with_models(&[]);
    run.start_ready()
}

#[given("the runtime is Waiting for models")]
fn runtime_waits_for_models(world: &mut IntegrationWorld) -> Result<()> {
    dictation_model_missing(world);
    let run = run(world);
    run.launch();
    run.waits_for_models()
}

#[when("the missing file is placed in the models folder")]
fn missing_file_placed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).place_missing_models()
}

#[when("the runtime reloads a configuration whose models folder is empty")]
fn reload_to_empty_models_folder(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let empty = ModelsFolder::new();
    let mut config = run.config.clone();
    config.services.models_dir = empty.folder();
    run.models = Some(empty);
    run.reload(config)
}

#[then("the pipeline ends and the observer is told Waiting for models")]
fn pipeline_ends_and_waits(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.pipeline_dropped(1)?;
    run.waits_for_models()
}

#[then("the observer is told Waiting for models")]
fn told_waiting_for_models(world: &mut IntegrationWorld) -> Result<()> {
    run(world).waits_for_models()
}

#[then("no service is started and no dictation pipeline runs")]
fn no_service_no_pipeline(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if run.any_service_required() {
        bail!("a service was started while models are missing");
    }
    run.no_pipeline_runs()
}

#[then("the services are started")]
fn services_are_started(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    if !run.any_service_required() {
        bail!("no service was started");
    }
    Ok(())
}

#[then("the observer is told Starting services and then Ready once the models are present")]
fn starting_services_then_ready(world: &mut IntegrationWorld) -> Result<()> {
    run(world).starting_services_then_ready()
}
