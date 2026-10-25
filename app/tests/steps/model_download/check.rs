//! Steps of the models check at launch.

use super::{
    bail, given, model_catalog, needs_of, runtime_run, then, when, Context, DownloadRun, Duration,
    FsModelStore, HashMap, IntegrationWorld, ModelStore, ModelsReport, Result, RuntimeState,
    DICTATION_MODEL,
};

pub(super) fn download(world: &mut IntegrationWorld) -> &mut DownloadRun {
    world.app.models.get_or_insert_with(DownloadRun::default)
}

// --- Missing models are listed before anything downloads

#[given("the app manages its services and the dictation model is not in the models folder")]
pub(super) fn app_misses_dictation_model(world: &mut IntegrationWorld) {
    crate::steps::permissions_gate::all_granted(world);
    runtime_run(world).manage_with_models(&[DICTATION_MODEL]);
}

#[given("every model file the configuration needs is in the models folder")]
pub(super) fn every_model_present(world: &mut IntegrationWorld) {
    crate::steps::permissions_gate::all_granted(world);
    runtime_run(world).manage_with_models(&[]);
}

#[when("the app launches")]
pub(super) fn app_launches(world: &mut IntegrationWorld) {
    runtime_run(world).launch();
}

/// The check the Download window shows, over the launched runtime's models folder.
pub(super) fn launched_check(world: &mut IntegrationWorld) -> ModelsReport {
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
pub(super) fn check_lists_dictation_missing(world: &mut IntegrationWorld) -> Result<()> {
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
pub(super) fn present_files_listed(world: &mut IntegrationWorld) -> Result<()> {
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
pub(super) fn nothing_downloaded_no_service(world: &mut IntegrationWorld) -> Result<()> {
    let run = runtime_run(world);
    std::thread::sleep(Duration::from_millis(300));
    if run.any_service_required() {
        bail!("a service was started while models are missing");
    }
    // the launch runs no download: only the Download window's button starts one
    if world.app.models.is_some() {
        bail!("a download ran");
    }
    Ok(())
}

#[then("the runtime never waits for models and the services start")]
pub(super) fn never_waits_services_start(world: &mut IntegrationWorld) -> Result<()> {
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
