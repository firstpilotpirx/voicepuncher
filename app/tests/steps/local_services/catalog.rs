//! The services a configuration needs, looked up by the catalog on a home of the scenario's
//! own: its models, its `$PATH` and, for a bundled app, its engine folders.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::localservices::catalog::{
    engine_origin, needed, Machine, Needs,
};
use voicepuncher::config::Config;
use voicepuncher::core::application::{ServiceLauncher, ServiceProcess, Services};
use voicepuncher::core::domain::{ServiceKind, ServiceSpec, ServiceState};

use super::home::{touch, FakeHome, COMMAND_MODEL_VAR, COMMAND_SPOTTING, KWS_MODEL_DIR, WHISPER};
use super::{local, Answering, StepClock};
use crate::IntegrationWorld;

/// What a lookup scenario looked up and on what.
#[derive(Debug)]
pub(super) struct CatalogRun {
    home: FakeHome,
    machine: Machine,
    pub(super) config: Config,
    specs: Vec<ServiceSpec>,
    /// The states of the services supervised after the lookup.
    states: Vec<(ServiceKind, ServiceState)>,
    spawns: usize,
    /// The copy ports of each service on its default address.
    pub(super) copy_ports: Vec<Vec<u16>>,
}

impl CatalogRun {
    fn new() -> Result<Self> {
        let home = FakeHome::complete()?;
        let machine = home.machine();
        let mut config = Config::default();
        config.voice_control.enabled = true;
        Ok(Self {
            home,
            machine,
            config,
            specs: Vec::new(),
            states: Vec::new(),
            spawns: 0,
            copy_ports: Vec::new(),
        })
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

#[given(
    "the variable for the speech recognition model for commands names a file that is not on disk"
)]
fn command_model_missing(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let missing = run.home.path().join("elsewhere/ggml-commands.bin");
    run.machine
        .env
        .insert(COMMAND_MODEL_VAR.into(), missing.display().to_string());
    Ok(())
}

#[given("two keyword spotting model folders in the models folder, the older one first by name")]
fn two_kws_models(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    std::fs::create_dir_all(run.home.models().join("sherpa-onnx-kws-zz-newer"))?;
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    std::fs::File::open(run.home.models().join(KWS_MODEL_DIR))?.set_modified(past)?;
    Ok(())
}

#[when("the services it needs are looked up")]
fn looked_up(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let needs = Needs::of(&run.config);
    run.specs = needed(&run.config, needs, &run.machine, &Answering::default());
    Ok(())
}

/// Refuses every start, counting them: a lookup scenario starts no program.
struct CountingLauncher(Arc<AtomicUsize>);

impl ServiceLauncher for CountingLauncher {
    fn spawn(&self, spec: &ServiceSpec) -> Result<Box<dyn ServiceProcess>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        bail!("{} is not started in this scenario", spec.label)
    }
}

#[when("the services it needs are looked up and supervised")]
fn looked_up_and_supervised(world: &mut IntegrationWorld) -> Result<()> {
    looked_up(world)?;
    let run = catalog(world)?;
    let spawns = Arc::new(AtomicUsize::new(0));
    let launcher = CountingLauncher(Arc::clone(&spawns));
    let mut services = Services::new(
        Box::new(launcher),
        Box::new(Answering::default()),
        Arc::new(StepClock::new()),
    );
    let commands = run
        .of_kind(ServiceKind::Commands)
        .into_iter()
        .cloned()
        .collect();
    services.supervise(commands);
    services.tick();
    run.states = services
        .snapshot()
        .into_iter()
        .map(|s| (s.kind, s.state))
        .collect();
    run.spawns = spawns.load(Ordering::SeqCst);
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

#[then("the speech recognition service for commands is Failed with a reason naming the model file it looked for")]
fn commands_failed_naming_model(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let missing = run
        .machine
        .env
        .get(COMMAND_MODEL_VAR)
        .context("no model was named")?;
    ensure!(!run.states.is_empty(), "no service was supervised");
    for (kind, state) in &run.states {
        let ServiceState::Failed(reason) = state else {
            bail!("the {} is {state}, not Failed", kind.name());
        };
        ensure!(reason.contains(missing.as_str()), "{reason}");
    }
    Ok(())
}

#[then("the app started no service")]
fn started_no_service(world: &mut IntegrationWorld) -> Result<()> {
    let spawns = catalog(world)?.spawns;
    ensure!(spawns == 0, "{spawns} service(s) were started");
    Ok(())
}

#[then("the command spotting service is started with the newer folder")]
fn started_with_newer_kws(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let newer = run
        .home
        .models()
        .join("sherpa-onnx-kws-zz-newer")
        .display()
        .to_string();
    for spec in run.of_kind(ServiceKind::CommandSpotting) {
        ensure!(
            spec.args.contains(&newer),
            "{} starts with {:?}",
            spec.label,
            spec.args
        );
    }
    Ok(())
}
