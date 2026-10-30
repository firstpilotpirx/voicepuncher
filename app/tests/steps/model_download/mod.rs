//! The model download over the app's models folder store in a temporary folder, a stand-in
//! host that serves, corrupts, refuses or stops mid-file, a disk with the free space a step
//! sets and a clock that only records its pauses; the launch scenarios drive the real runtime
//! of the app-runtime steps with the app's catalog gate.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use voicepuncher::adapters::secondary::modelfetch::fs::FsModelStore;
use voicepuncher::core::application::{
    Clock, DiskSpace, DownloadEvent, DownloadFailure, DownloadSummary, FetchError, ModelDownload,
    ModelFetcher, ModelStore, ModelsReport, PartSink,
};
use voicepuncher::core::domain::model_catalog::{self, ModelFile, ModelKind, ModelUse};
use voicepuncher::runtime::models::needs_of;
use voicepuncher::runtime::RuntimeState;

use crate::steps::app_runtime::{models::DICTATION_MODEL, run as runtime_run};
use crate::IntegrationWorld;

pub mod files;

mod host;
pub(crate) use host::*;
mod check;
use check::{app_misses_dictation_model, download};
mod downloads;
mod recognizer;

const GB: u64 = 1_000_000_000;
