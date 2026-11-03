//! The services a configuration needs, looked up by the catalog on a home of the scenario's
//! own: its models, its `$PATH` and, for a bundled app, its engine folders.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::localservices::catalog::{
    engine_origin, needed, Machine, Needs,
};
use voicepuncher::config::Config;
use voicepuncher::core::domain::{ServiceKind, ServiceSpec};

use super::home::{touch, FakeHome, COMMAND_SPOTTING, WHISPER};
use super::{local, Answering};
use crate::IntegrationWorld;

/// What a lookup scenario looked up and on what.
#[derive(Debug)]
pub(super) struct CatalogRun {
    home: FakeHome,
    machine: Machine,
    pub(super) config: Config,
    specs: Vec<ServiceSpec>,
    /// The copy ports of each service on its default address.
    pub(super) copy_ports: Vec<Vec<u16>>,
}

impl CatalogRun {
    fn new() -> Result<Self> {
        let home = FakeHome::complete()?;
        let machine = home.machine();
        let mut config = Config::default();
        config.voice_control.enabled = true;
        Ok(Self { home, machine, config, specs: Vec::new(), copy_ports: Vec::new() })
    }

    /// The scenario's home folder.
    pub(super) fn home(&self) -> &Path {
        &self.machine.home
    }

    pub(super) fn of_kind(&self, kind: ServiceKind) -> Vec<&ServiceSpec> {
        self.specs.iter().filter(|spec| spec.kind == kind).collect()
    }
}

pub(super) fn catalog(world: &mut IntegrationWorld) -> Result<&mut CatalogRun> {
    let local = local(world);
    if local.catalog.is_none() {
        local.catalog = Some(CatalogRun::new()?);
    }
    local.catalog.as_mut().context("a lookup scenario")
}

/// Puts every engine into its folder of the bundle next to the app.
fn bundle_every_engine(run: &CatalogRun) -> Result<()> {
    for (folder, program) in [("whisper", WHISPER), ("command-spotting", COMMAND_SPOTTING)] {
        touch(&run.home.engines().join(folder).join(program))?;
    }
    Ok(())
}

#[given(
    "the app runs from a bundle whose engine folders hold every engine, and $PATH holds them too"
)]
fn bundle_and_path(world: &mut IntegrationWorld) -> Result<()> {
    bundle_every_engine(catalog(world)?)
}

#[given("the app runs from a build folder and $PATH holds every engine")]
#[given(
    "no local service runs, voice control is on and the number of its copies is not configured"
)]
fn every_engine_on_path(world: &mut IntegrationWorld) -> Result<()> {
    catalog(world).map(|_| ())
}

#[given(
    expr = "the app runs from a bundle whose speech recognition engine folder has the version file {word}"
)]
fn bundled_engine_version(world: &mut IntegrationWorld, version: String) -> Result<()> {
    let run = catalog(world)?;
    bundle_every_engine(run)?;
    let folder = run.home.engines().join("whisper");
    std::fs::write(folder.join("VERSION"), format!("{version}\n"))?;
    Ok(())
}

#[given(
    expr = "the app runs outside a bundle and $PATH leads to whisper-server in Homebrew's folder of version {word}"
)]
fn homebrew_engine_version(world: &mut IntegrationWorld, version: String) -> Result<()> {
    let run = catalog(world)?;
    let installed = run
        .home
        .path()
        .join("Cellar/whisper-cpp")
        .join(version)
        .join("bin");
    touch(&installed.join(WHISPER))?;
    let linked = run.home.bin().join(WHISPER);
    std::fs::remove_file(&linked)?;
    std::os::unix::fs::symlink(installed.join(WHISPER), linked)?;
    Ok(())
}

#[when("the services it needs are looked up")]
fn looked_up(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let needs = Needs::of(&run.config);
    run.specs = needed(&run.config, needs, &run.machine, &Answering::default());
    Ok(())
}

#[then("each service's program is the one in its engine folder inside the bundle")]
fn programs_from_bundle(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    programs_in(run, &run.home.engines())
}

#[then("each service's program is the one on $PATH")]
fn programs_from_path(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    programs_in(run, &run.home.bin())
}

fn programs_in(run: &CatalogRun, folder: &Path) -> Result<()> {
    ensure!(!run.specs.is_empty(), "no service was looked up");
    for spec in &run.specs {
        ensure!(
            spec.program.starts_with(folder),
            "{} runs {}, not one in {}",
            spec.label,
            spec.program.display(),
            folder.display()
        );
    }
    Ok(())
}

#[then(regex = r"^the speech recognition service is named as running (.+)$")]
fn engine_named(world: &mut IntegrationWorld, named: String) -> Result<()> {
    let run = catalog(world)?;
    let dictation = run.of_kind(ServiceKind::Dictation);
    let spec = dictation
        .first()
        .context("no speech recognition service was looked up")?;
    let origin = engine_origin(&spec.program, &run.machine).to_string();
    ensure!(origin == named, "the log names {origin:?}, not {named:?}");
    Ok(())
}
