//! Where a model counts as present, checked by the app's catalog gate over a home of its own,
//! and the headless download, run as the app's own command line with every address on a port
//! that refuses connections.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::config::Config;
use voicepuncher::core::domain::{ModelFile, ModelKind, CATALOG};
use voicepuncher::runtime::models::MIRROR_VARIABLE;
use voicepuncher::runtime::{CatalogModels, ModelsGate};

use crate::steps::app_runtime::models::DICTATION_MODEL;
use crate::IntegrationWorld;

/// The catalog file the failing headless download misses.
const REFUSED_FILE: &str = "silero_vad.onnx";

/// What a scenario of the model files shares between its steps.
#[derive(Debug)]
pub struct ModelFilesRun {
    home: TempDir,
    env: HashMap<String, String>,
    missing: Option<Vec<&'static str>>,
    output: Option<Output>,
}

impl Default for ModelFilesRun {
    fn default() -> Self {
        Self {
            home: tempfile::tempdir().expect("a temporary home"),
            env: HashMap::new(),
            missing: None,
            output: None,
        }
    }
}

impl ModelFilesRun {
    fn folder(&self) -> PathBuf {
        self.home.path().join("models")
    }

    /// Puts every catalog file but `missing` in the models folder at its catalog size.
    fn fill(&self, missing: Option<&str>) -> Result<()> {
        for file in CATALOG.iter().filter(|file| Some(file.name) != missing) {
            place(&self.folder(), file, file.size_bytes)?;
        }
        Ok(())
    }

    fn printed(&self) -> Result<(String, Option<i32>)> {
        let output = self.output.as_ref().context("the download did not run")?;
        let printed = String::from_utf8_lossy(&output.stderr).into_owned();
        Ok((printed, output.status.code()))
    }
}

/// Puts `file` in `folder` at `size` bytes.
pub(crate) fn place(folder: &Path, file: &ModelFile, size: u64) -> Result<()> {
    std::fs::create_dir_all(folder)?;
    match file.kind {
        ModelKind::File => std::fs::File::create(folder.join(file.name))
            .and_then(|placed| placed.set_len(size))
            .context("a model file"),
        ModelKind::TarBz2 { folder: name } => {
            std::fs::create_dir_all(folder.join(name)).context("a model folder")
        }
    }
}

fn dictation_model() -> &'static ModelFile {
    CATALOG
        .iter()
        .find(|file| file.name == DICTATION_MODEL)
        .expect("the catalog holds the dictation model")
}

fn files(world: &mut IntegrationWorld) -> &mut ModelFilesRun {
    world
        .extra
        .ui
        .model_files
        .get_or_insert_with(ModelFilesRun::default)
}

#[given("the dictation model file in the models folder is shorter than the catalog size")]
fn dictation_model_short(world: &mut IntegrationWorld) -> Result<()> {
    let run = files(world);
    let model = dictation_model();
    place(&run.folder(), model, model.size_bytes - 1)
}

#[given("the variable for the dictation model names an existing file outside the models folder")]
fn dictation_model_by_variable(world: &mut IntegrationWorld) -> Result<()> {
    let run = files(world);
    let elsewhere = run.home.path().join("elsewhere.bin");
    std::fs::write(&elsewhere, b"a model kept elsewhere")?;
    let variable = dictation_model().variable.to_owned();
    run.env.insert(variable, elsewhere.display().to_string());
    Ok(())
}

#[given("a folder that holds every catalog file")]
fn folder_with_every_file(world: &mut IntegrationWorld) -> Result<()> {
    files(world).fill(None)
}

#[given("a folder that misses one catalog file and a host for it that refuses connections")]
fn folder_missing_one(world: &mut IntegrationWorld) -> Result<()> {
    files(world).fill(Some(REFUSED_FILE))
}

#[when("the app checks the models")]
fn app_checks_models(world: &mut IntegrationWorld) {
    let run = files(world);
    let mut config = Config::default();
    config.services.models_dir = run.folder();
    let gate = CatalogModels::new(run.home.path(), run.env.clone());
    run.missing = Some(gate.missing(&config));
}

/// An address on this machine where nothing listens.
fn refused_mirror() -> Result<String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(format!(
        "http://127.0.0.1:{}",
        listener.local_addr()?.port()
    ))
}

#[when("the user runs the headless download of the models into that folder")]
fn headless_download(world: &mut IntegrationWorld) -> Result<()> {
    let run = files(world);
    let mut command = Command::new(env!("CARGO_BIN_EXE_voicepuncher"));
    command
        .args(["--download-models", "--models-dir"])
        .arg(run.folder())
        .env("HOME", run.home.path())
        .env(MIRROR_VARIABLE, refused_mirror()?);
    for file in CATALOG {
        command.env_remove(file.variable);
    }
    run.output = Some(command.output()?);
    Ok(())
}

#[then(regex = r"^the dictation model is (Missing|Present)$")]
fn dictation_model_is(world: &mut IntegrationWorld, wanted: String) -> Result<()> {
    let missing = files(world)
        .missing
        .clone()
        .context("the models were not checked")?;
    let is = if missing.contains(&DICTATION_MODEL) {
        "Missing"
    } else {
        "Present"
    };
    if is != wanted {
        bail!("the dictation model is {is}; missing: {missing:?}");
    }
    Ok(())
}

#[then("nothing is fetched and every file is reported present")]
fn nothing_fetched(world: &mut IntegrationWorld) -> Result<()> {
    let (printed, _) = files(world).printed()?;
    if printed.contains("downloading") || !printed.contains("every model is present") {
        bail!("the download printed:\n{printed}");
    }
    Ok(())
}

#[then(regex = r"^the command exits (\d)$")]
fn command_exits(world: &mut IntegrationWorld, code: i32) -> Result<()> {
    let (printed, exit) = files(world).printed()?;
    if exit != Some(code) {
        bail!("the command exited {exit:?}:\n{printed}");
    }
    Ok(())
}

#[then("the command names the failed file")]
fn names_failed_file(world: &mut IntegrationWorld) -> Result<()> {
    let (printed, _) = files(world).printed()?;
    let wanted = format!("{REFUSED_FILE}: failed: no internet connection");
    if !printed.contains(&wanted) {
        bail!("the download did not name {REFUSED_FILE}:\n{printed}");
    }
    Ok(())
}
