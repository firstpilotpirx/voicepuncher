//! The model download over the app's models folder store in a temporary folder, a stand-in
//! host that serves, corrupts, refuses or stops mid-file, a disk with the free space a step
//! sets and a clock that only records its pauses; the launch scenarios drive the real runtime
//! of the app-runtime steps with the app's catalog gate.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
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

const GB: u64 = 1_000_000_000;

/// How the stand-in host answers for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Serve,
    /// Bytes that never match the checksum.
    Corrupt,
    Refuse,
    /// Stops after this many bytes, as Cancel does.
    CancelAfter(usize),
}

/// Every request: the file and the byte it asked to start from.
type Requests = Arc<Mutex<Vec<(String, u64)>>>;

#[derive(Clone)]
struct Host {
    bodies: HashMap<String, Vec<u8>>,
    answers: Arc<Mutex<HashMap<String, Answer>>>,
    requests: Requests,
}

impl ModelFetcher for Host {
    fn fetch(
        &self,
        url: &str,
        sink: &mut dyn PartSink,
        received: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), FetchError> {
        let from = sink.len();
        self.requests.lock().unwrap().push((url.to_string(), from));
        let body = self.bodies.get(url).cloned().unwrap_or_default();
        let answer = self
            .answers
            .lock()
            .unwrap()
            .get(url)
            .copied()
            .unwrap_or(Answer::Serve);
        let rest = &body[from as usize..];
        match answer {
            Answer::Refuse => Err(FetchError::Connection("connection refused".into())),
            Answer::Corrupt => {
                let wrong: Vec<u8> = rest.iter().map(|byte| byte ^ 0xff).collect();
                sink.write(&wrong).map_err(FetchError::Disk)?;
                received(sink.len());
                Ok(())
            }
            Answer::CancelAfter(bytes) => {
                sink.write(&rest[..bytes]).map_err(FetchError::Disk)?;
                received(sink.len());
                cancel.store(true, Ordering::SeqCst);
                Err(FetchError::Cancelled)
            }
            Answer::Serve => {
                for chunk in rest.chunks(64) {
                    sink.write(chunk).map_err(FetchError::Disk)?;
                    received(sink.len());
                }
                Ok(())
            }
        }
    }
}

struct Disk(Option<u64>);

impl DiskSpace for Disk {
    fn free_bytes(&self) -> Option<u64> {
        self.0
    }
}

/// Records its pauses without sleeping.
#[derive(Clone, Default)]
struct PauseClock(Arc<Mutex<Vec<Duration>>>);

impl Clock for PauseClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, pause: Duration) {
        self.0.lock().unwrap().push(pause);
    }
}

/// What a download scenario shares with its steps.
pub struct DownloadRun {
    home: TempDir,
    files: Vec<ModelFile>,
    host: Host,
    free: Option<u64>,
    clock: PauseClock,
    events: Vec<DownloadEvent>,
    summary: Option<DownloadSummary>,
}

impl std::fmt::Debug for DownloadRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadRun")
            .field("events", &self.events)
            .finish()
    }
}

impl Default for DownloadRun {
    fn default() -> Self {
        Self {
            home: tempfile::tempdir().expect("a temporary home"),
            files: Vec::new(),
            host: Host {
                bodies: HashMap::new(),
                answers: Arc::default(),
                requests: Arc::default(),
            },
            free: None,
            clock: PauseClock::default(),
            events: Vec::new(),
            summary: None,
        }
    }
}

fn leaked(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl DownloadRun {
    fn folder(&self) -> PathBuf {
        self.home.path().join("models")
    }

    /// A catalog file of `size` bytes, served by the host.
    fn add_file(&mut self, size: usize) -> ModelFile {
        let index = self.files.len();
        let body: Vec<u8> = (0..size).map(|i| (i * 7 + index) as u8).collect();
        let name = leaked(format!("model-{index}.bin"));
        let file = ModelFile {
            name,
            url: leaked(format!("https://models.test/{name}")),
            sha256: leaked(sha256_hex(&body)),
            size_bytes: size as u64,
            purpose: "test model",
            kind: ModelKind::File,
            used_by: ModelUse::Dictation,
            variable: leaked(format!("VOICEPUNCHER_TEST_MODEL_{index}")),
        };
        self.host.bodies.insert(file.url.to_string(), body);
        self.files.push(file);
        file
    }

    /// A catalog file of `bytes` that is never fetched.
    fn add_unfetched(&mut self, bytes: u64) {
        let index = self.files.len();
        self.files.push(ModelFile {
            name: leaked(format!("big-{index}.bin")),
            url: leaked(format!("https://models.test/big-{index}.bin")),
            sha256: "0",
            size_bytes: bytes,
            purpose: "test model",
            kind: ModelKind::File,
            used_by: ModelUse::Dictation,
            variable: leaked(format!("VOICEPUNCHER_TEST_MODEL_{index}")),
        });
    }

    fn answer(&self, file: &ModelFile, answer: Answer) {
        self.host
            .answers
            .lock()
            .unwrap()
            .insert(file.url.to_string(), answer);
    }

    fn body(&self, file: &ModelFile) -> &[u8] {
        &self.host.bodies[file.url]
    }

    fn place(&self, file: &ModelFile) {
        std::fs::create_dir_all(self.folder()).unwrap();
        std::fs::write(self.folder().join(file.name), self.body(file)).unwrap();
    }

    fn write_part(&self, file: &ModelFile, bytes: &[u8]) {
        std::fs::create_dir_all(self.folder()).unwrap();
        std::fs::write(self.folder().join(format!("{}.part", file.name)), bytes).unwrap();
    }

    fn store(&self) -> FsModelStore {
        FsModelStore::new(self.folder(), self.home.path(), HashMap::new())
    }

    fn download(&self) -> ModelDownload {
        ModelDownload::new(
            self.files.clone(),
            Box::new(self.store()),
            Box::new(self.host.clone()),
            Box::new(Disk(self.free)),
            Arc::new(self.clock.clone()),
        )
    }

    fn check(&self) -> ModelsReport {
        self.download().check()
    }

    fn run_download(&mut self) {
        let mut events = Vec::new();
        let summary = self.download().download_missing(
            &mut |event: &DownloadEvent| events.push(event.clone()),
            &AtomicBool::new(false),
        );
        self.events.extend(events);
        self.summary = Some(summary);
    }

    fn requests(&self) -> Vec<(String, u64)> {
        self.host.requests.lock().unwrap().clone()
    }

    fn requests_of(&self, file: &ModelFile) -> Vec<u64> {
        self.requests()
            .into_iter()
            .filter(|(url, _)| url == file.url)
            .map(|(_, from)| from)
            .collect()
    }

    fn placed_matches(&self, file: &ModelFile) -> Result<()> {
        let placed = std::fs::read(self.folder().join(file.name))
            .with_context(|| format!("{} is not placed", file.name))?;
        if sha256_hex(&placed) != file.sha256 {
            bail!("{} was placed with another checksum", file.name);
        }
        Ok(())
    }

    fn file(&self, index: usize) -> ModelFile {
        self.files[index]
    }
}

fn download(world: &mut IntegrationWorld) -> &mut DownloadRun {
    world.models.get_or_insert_with(DownloadRun::default)
}

// --- Missing models are listed before anything downloads

#[given("the app manages its services and the dictation model is not in the models folder")]
fn app_misses_dictation_model(world: &mut IntegrationWorld) {
    crate::steps::permissions_gate::all_granted(world);
    runtime_run(world).manage_with_models(&[DICTATION_MODEL]);
}

#[given("every model file the configuration needs is in the models folder")]
fn every_model_present(world: &mut IntegrationWorld) {
    crate::steps::permissions_gate::all_granted(world);
    runtime_run(world).manage_with_models(&[]);
}

#[when("the app launches")]
fn app_launches(world: &mut IntegrationWorld) {
    runtime_run(world).launch();
}

/// The check the Download window shows, over the launched runtime's models folder.
fn launched_check(world: &mut IntegrationWorld) -> ModelsReport {
    let run = runtime_run(world);
    let config = run.config().clone();
    let home = run.models_home().expect("the runtime waits for models");
    let files = model_catalog::needed(needs_of(&config));
    let store = FsModelStore::new(
        config.services.resolved_models_dir(&home),
        &home,
        HashMap::new(),
    );
    let rows = files
        .iter()
        .map(|file| voicepuncher::core::application::ModelRow {
            file: *file,
            present: store.present(file),
            part: 0,
        })
        .collect();
    ModelsReport { rows, missing_bytes: 0, free_bytes: None }
}

#[then("the models check lists the dictation model as Missing with its size and purpose")]
fn check_lists_dictation_missing(world: &mut IntegrationWorld) -> Result<()> {
    runtime_run(world).waits_for_models()?;
    let report = launched_check(world);
    let row = report
        .rows
        .iter()
        .find(|row| row.file.name == DICTATION_MODEL)
        .context("the dictation model is not listed")?;
    if row.present || row.file.size_bytes == 0 || row.file.purpose.is_empty() {
        bail!("the dictation model row is {row:?}");
    }
    Ok(())
}

#[then("the files that are present are listed as Present")]
fn present_files_listed(world: &mut IntegrationWorld) -> Result<()> {
    let report = launched_check(world);
    let wrong: Vec<_> = report
        .rows
        .iter()
        .filter(|row| (row.file.name == DICTATION_MODEL) == row.present)
        .map(|row| row.file.name)
        .collect();
    if !wrong.is_empty() {
        bail!("listed wrongly: {wrong:?}");
    }
    Ok(())
}

#[then("nothing is downloaded and no service is started")]
fn nothing_downloaded_no_service(world: &mut IntegrationWorld) -> Result<()> {
    let run = runtime_run(world);
    std::thread::sleep(Duration::from_millis(300));
    if run.any_service_required() {
        bail!("a service was started while models are missing");
    }
    // the launch runs no download: only the Download window's button starts one
    if world.models.is_some() {
        bail!("a download ran");
    }
    Ok(())
}

#[then("the runtime never waits for models and the services start")]
fn never_waits_services_start(world: &mut IntegrationWorld) -> Result<()> {
    let run = runtime_run(world);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    if run.ever_waited_for_models() {
        bail!("the runtime waited for models");
    }
    if !run.any_service_required() {
        bail!("no service was started");
    }
    Ok(())
}

// --- Download fetches, verifies and places each file

#[given("two needed files are missing")]
fn two_missing(world: &mut IntegrationWorld) {
    let run = download(world);
    let present = run.add_file(100);
    run.place(&present);
    run.add_file(300);
    run.add_file(500);
}

#[when("the user chooses Download")]
#[when("the user chooses Download again")]
#[when("the file is downloaded")]
fn user_chooses_download(world: &mut IntegrationWorld) {
    download(world).run_download();
}

#[then(
    "each file is fetched in turn with its progress shown, its checksum verified, and moved to its name in the models folder"
)]
fn each_fetched_in_turn(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let (first, second) = (run.file(1), run.file(2));
    let order: Vec<&str> = run
        .events
        .iter()
        .filter_map(|event| match event {
            DownloadEvent::Started(name) | DownloadEvent::Done(name) => Some(*name),
            _ => None,
        })
        .collect();
    if order != [first.name, first.name, second.name, second.name] {
        bail!("not fetched in turn: {:?}", run.events);
    }
    for file in [first, second] {
        let progress = run.events.iter().any(|event| {
            matches!(event, DownloadEvent::Progress { name, received, total, .. }
                if *name == file.name && *received == file.size_bytes && *total == file.size_bytes)
        });
        if !progress {
            bail!("no progress shown for {}: {:?}", file.name, run.events);
        }
        run.placed_matches(&file)?;
    }
    if run.requests_of(&run.file(0)) != Vec::<u64>::new() {
        bail!("the present file was fetched");
    }
    Ok(())
}

#[then("no part file is left")]
fn no_part_left(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let parts: Vec<_> = std::fs::read_dir(run.folder())?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".part"))
        .collect();
    if !parts.is_empty() {
        bail!("parts left: {parts:?}");
    }
    Ok(())
}

// --- A download resumes where it stopped

#[given("a file was half downloaded when the user chose Cancel")]
fn half_downloaded_then_cancelled(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.add_file(1000);
    run.answer(&file, Answer::CancelAfter(500));
    run.run_download();
    if !run
        .summary
        .as_ref()
        .is_some_and(|summary| summary.cancelled)
    {
        bail!("the download was not cancelled: {:?}", run.events);
    }
    run.answer(&file, Answer::Serve);
    run.host.requests.lock().unwrap().clear();
    Ok(())
}

#[then("only the remaining bytes are fetched")]
fn only_remaining_fetched(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.file(0);
    if run.requests_of(&file) != [500] {
        bail!("requests: {:?}", run.requests());
    }
    Ok(())
}

#[then("the placed file's checksum matches the catalog")]
fn placed_checksum_matches(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    run.placed_matches(&run.file(0))
}

// --- A file whose checksum does not match is downloaded again

#[given("a part whose bytes were changed")]
fn corrupted_part(world: &mut IntegrationWorld) {
    let run = download(world);
    let file = run.add_file(800);
    let corrupted: Vec<u8> = run.body(&file)[..400].iter().map(|b| b ^ 0x55).collect();
    run.write_part(&file, &corrupted);
}

#[then(
    "the mismatch is found, the file is fetched again from the start and placed with a matching checksum"
)]
fn mismatch_found_and_refetched(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.file(0);
    if !run
        .events
        .contains(&DownloadEvent::MismatchRetrying(file.name))
    {
        bail!("no mismatch found: {:?}", run.events);
    }
    if run.requests_of(&file) != [400, 0] {
        bail!("requests: {:?}", run.requests());
    }
    run.placed_matches(&file)
}

#[given("the server sends bytes that never match the catalog checksum")]
fn never_matches(world: &mut IntegrationWorld) {
    let run = download(world);
    let file = run.add_file(600);
    run.answer(&file, Answer::Corrupt);
}

#[then("the file is fetched twice and then reported Failed with checksum mismatch")]
fn fetched_twice_then_failed(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let file = run.file(0);
    if run.requests_of(&file) != [0, 0] {
        bail!("requests: {:?}", run.requests());
    }
    let failed =
        DownloadEvent::Failed { name: file.name, failure: DownloadFailure::ChecksumMismatch };
    if !run.events.contains(&failed) {
        bail!(
            "not reported Failed with checksum mismatch: {:?}",
            run.events
        );
    }
    Ok(())
}

#[then("nothing is placed under its name")]
fn nothing_placed(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let placed = run.folder().join(run.file(0).name);
    if placed.exists() {
        bail!("{} was placed", placed.display());
    }
    Ok(())
}

// --- Without internet the user is told and nothing is deleted

#[given("no connection can be made to the model hosts")]
fn no_connection(world: &mut IntegrationWorld) {
    let run = download(world);
    let present = run.add_file(100);
    run.place(&present);
    for size in [200, 300] {
        let file = run.add_file(size);
        run.answer(&file, Answer::Refuse);
    }
}

#[then(
    "each missing file is tried 4 times with pauses of 2, 4 and 8 seconds and then reported Failed with no internet connection"
)]
fn tried_four_times(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let pauses: Vec<u64> = run
        .clock
        .0
        .lock()
        .unwrap()
        .iter()
        .map(Duration::as_secs)
        .collect();
    if pauses != [2, 4, 8, 2, 4, 8] {
        bail!("pauses: {pauses:?}");
    }
    for file in [run.file(1), run.file(2)] {
        if run.requests_of(&file).len() != 4 {
            bail!("{} tried {:?}", file.name, run.requests_of(&file));
        }
        let failed =
            DownloadEvent::Failed { name: file.name, failure: DownloadFailure::NoInternet };
        if !run.events.contains(&failed) {
            bail!(
                "{} not reported Failed with no internet connection",
                file.name
            );
        }
        if DownloadFailure::NoInternet.to_string() != "no internet connection" {
            bail!("the failure reads {}", DownloadFailure::NoInternet);
        }
    }
    Ok(())
}

#[then("the files that were present are still present")]
fn present_still_present(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    run.placed_matches(&run.file(0))?;
    if !run.check().rows[0].present {
        bail!("the present file is no longer present");
    }
    Ok(())
}

// --- Not enough space is refused before starting

#[given("3 GB are missing and 1 GB is free")]
fn three_missing_one_free(world: &mut IntegrationWorld) {
    let run = download(world);
    run.add_unfetched(2 * GB);
    run.add_unfetched(GB);
    run.free = Some(GB);
}

#[then("nothing is fetched")]
fn nothing_fetched(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    if !run.requests().is_empty() {
        bail!("fetched: {:?}", run.requests());
    }
    Ok(())
}

#[then("the download reports that 3 GB are needed and 1 GB is free")]
fn reports_needed_and_free(world: &mut IntegrationWorld) -> Result<()> {
    let run = download(world);
    let refused = DownloadEvent::NotEnoughSpace { needed: 3 * GB, free: GB };
    if run.events != [refused] {
        bail!("events: {:?}", run.events);
    }
    Ok(())
}

// --- The services start when the last file is present

#[given("the runtime waits for models")]
fn runtime_waits(world: &mut IntegrationWorld) -> Result<()> {
    app_misses_dictation_model(world);
    let run = runtime_run(world);
    run.launch();
    run.waits_for_models()
}

#[when("the last missing file is downloaded and placed")]
fn last_file_placed(world: &mut IntegrationWorld) -> Result<()> {
    runtime_run(world).place_missing_models()
}

#[then("the runtime starts the services and becomes Ready once they answer")]
fn services_start_and_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = runtime_run(world);
    run.starting_services_then_ready()?;
    if !run.any_service_required() {
        bail!("no service was started");
    }
    Ok(())
}
