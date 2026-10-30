//! The `e2e-bundle` stage: a copy of the bundle the bundle stage built, launched outside the
//! build folder with nothing but the system folders on its path, so every service it starts runs
//! the bundle's own engines; then one transcription and the command spotting
//! service's health through them, and the engines' logs read for the Metal GPU.

use std::fs;
use std::net::TcpListener;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use voicepuncher::adapters::secondary::logging::file::{logs_dir, LOG_FILE_NAME};
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::TranscriptionCfg;
use voicepuncher_copy_pool::ServiceAddress;
use voicepuncher_e2e_test::corpus::load_manifest;
use voicepuncher_e2e_test::speech::read_wav;

use super::bundled::program;
use super::{copy_bundle, stage_bundle, BUNDLE_DIR};
use crate::steps::off_thread;
use crate::E2eWorld;

mod launched_copy;

/// The models the services need, as the `justfile` exports them; the only variables the copy
/// gets beside its home and its path.
const MODELS: [&str; 5] = [
    "VOICEPUNCHER_WHISPER_MODEL",
    "VOICEPUNCHER_VAD_MODEL",
    "VOICEPUNCHER_COMMAND_WHISPER_MODEL",
    "VOICEPUNCHER_KWS_MODEL_DIR",
    "VOICEPUNCHER_KWS_VAD_MODEL",
];
const CORPUS: &str = "VOICEPUNCHER_BENCH_CORPUS";
const APP_PROGRAM: &str = "Contents/MacOS/voicepuncher";
/// The system folders a Mac without Homebrew has on its path.
const SYSTEM_PATH: &str = "/usr/bin:/bin";
/// The engine sections of the lock, by the program each runs.
const ENGINES: [&str; 2] = ["whisper", "command-spotting"];
/// What the app logs for an engine it takes from the bundle.
const FROM_BUNDLE: &str = "from the bundle";
/// What whisper-server logs when it sets up the Metal device and its library; a line of them
/// that says error or failed is a Metal failure.
const WHISPER_METAL: &str = "ggml_metal_library_init: using";
const METAL_PREFIX: &str = "ggml_metal";
const METAL_FAILURES: [&str; 2] = ["error", "failed"];
/// What ggml logs for each backend it loads, followed by the file it loaded.
const BACKEND_LOADED: &str = "load_backend: loaded";
/// Loading the dictation models takes 5–30 s; the app gives each start 60 s.
const READY_WITHIN: Duration = Duration::from_secs(120);
/// The app asks its services to end and kills one after 5 s.
const ENDED_WITHIN: Duration = Duration::from_secs(15);

/// The copy, its home and the app it runs; the app is told to end when the scenario ends.
#[derive(Debug, Default)]
pub struct BundleLaunch {
    folder: Option<tempfile::TempDir>,
    home: Option<tempfile::TempDir>,
    ports: Ports,
    child: Option<Child>,
    ready: Vec<(String, bool)>,
}

#[derive(Debug, Default, Clone, Copy)]
struct Ports {
    stt: u16,
    command_stt: u16,
    kws: u16,
}

impl Drop for BundleLaunch {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = Command::new("kill")
                .args(["-TERM", &child.id().to_string()])
                .status();
            let deadline = Instant::now() + ENDED_WITHIN;
            while Instant::now() < deadline && matches!(child.try_wait(), Ok(None)) {
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl BundleLaunch {
    fn logs(&self) -> Result<PathBuf> {
        Ok(logs_dir(
            self.home
                .as_ref()
                .context("the copy was not launched")?
                .path(),
        ))
    }

    fn app_log(&self) -> String {
        self.logs()
            .and_then(|dir| Ok(fs::read_to_string(dir.join(LOG_FILE_NAME))?))
            .unwrap_or_default()
    }

    /// The log of the engine `program` on `port`, as the app names it.
    fn engine_log(&self, program: &str, port: u16) -> Result<String> {
        let path = self.logs()?.join(format!("{program}-{port}.log"));
        fs::read_to_string(&path).with_context(|| format!("{} is missing", path.display()))
    }
}

fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn port_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// A free port for a speech recognition service whose copy ports are free too.
fn free_speech_recognition_port() -> Result<u16> {
    loop {
        let port = free_port()?;
        let copies = ServiceAddress::parse(&url(port))
            .map_err(|e| anyhow!("{e}"))?
            .copy_ports();
        if copies.into_iter().all(port_free) {
            return Ok(port);
        }
    }
}

fn url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/v1")
}

fn healthy(port: u16) -> bool {
    ureq::get(&format!("http://127.0.0.1:{port}/health"))
        .timeout(Duration::from_secs(2))
        .call()
        .is_ok()
}

fn launch(world: &mut E2eWorld) -> &mut BundleLaunch {
    &mut world.distribution.launch
}

#[given(
    "a copy of the bundle outside the build folder, the models on disk and nothing but the system folders on the path"
)]
fn bundle_copy(world: &mut E2eWorld) -> Result<()> {
    let bundle = stage_bundle().context("run 'just bundle' first, or 'just bundle-smoke'")?;
    for variable in MODELS.iter().chain([&CORPUS]) {
        let path = std::env::var(variable).with_context(|| format!("{variable} is not set"))?;
        ensure!(
            Path::new(&path).exists(),
            "{variable} names nothing: {path} — run 'just models'"
        );
    }
    let folder = tempfile::tempdir()?;
    copy_bundle(&bundle, folder.path())?;
    let launch = launch(world);
    launch.folder = Some(folder);
    launch.ports = Ports {
        stt: free_speech_recognition_port()?,
        command_stt: free_speech_recognition_port()?,
        kws: free_port()?,
    };
    Ok(())
}

#[when("the copy's application starts its services with voice control on")]
async fn copy_starts_services(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let launch = launch(world);
        let app = launch
            .folder
            .as_ref()
            .context("no copy")?
            .path()
            .join(BUNDLE_DIR)
            .join(APP_PROGRAM);
        let home = tempfile::tempdir()?;
        let config = write_config(home.path(), launch.ports)?;
        let mut command = app_command(&app, &config, home.path())?;
        launch.child = Some(command.spawn().context("the copy's app could not start")?);
        launch.home = Some(home);
        launch.wait_until_ready()
    })
    .await
}

/// The copy's configuration in `home`: one copy of each service, so each answers on the port
/// the scenario checks.
fn write_config(home: &Path, ports: Ports) -> Result<PathBuf> {
    let Ports { stt, command_stt, kws } = ports;
    let config = home.join("config.toml");
    fs::write(
        &config,
        format!(
            "[transcription]\nbase_url = \"{}\"\n\n\
             [voice_control]\nenabled = true\nurl = \"ws://127.0.0.1:{kws}\"\n\
             recognizer = \"keyword_spotting\"\n\n\
             [services]\nmanage = true\ndictation_copies = 1\ncommand_copies = 1\n\
             command_spotting_copies = 1\n\
             commands_url = \"{}\"\n",
            url(stt),
            url(command_stt)
        ),
    )?;
    Ok(config)
}

/// The copy's app with nothing of the developer's environment: no Homebrew on the path, no
/// `*_BINARY`, no `DYLD_*`; only the models.
fn app_command(app: &Path, config: &Path, home: &Path) -> Result<Command> {
    let mut command = Command::new(app);
    command
        .arg("--config")
        .arg(config)
        .env_clear()
        .env("HOME", home)
        .env("PATH", SYSTEM_PATH)
        .current_dir(home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for variable in MODELS {
        command.env(variable, std::env::var(variable)?);
    }
    Ok(command)
}

impl BundleLaunch {
    /// Waits up to [`READY_WITHIN`] for every service the app starts to be healthy, and notes
    /// which were; fails when the app ends first.
    fn wait_until_ready(&mut self) -> Result<()> {
        let Ports { stt, command_stt, kws } = self.ports;
        let services = [
            ("speech recognition", stt),
            ("speech recognition for commands", command_stt),
            ("command spotting", kws),
        ];
        let deadline = Instant::now() + READY_WITHIN;
        let mut ready = vec![false; services.len()];
        while Instant::now() < deadline && ready.contains(&false) {
            if let Some(status) = self.child.as_mut().context("no app")?.try_wait()? {
                bail!(
                    "the copy's app ended with {status}; its log:\n{}",
                    self.app_log()
                );
            }
            for (index, (_, port)) in services.iter().enumerate() {
                ready[index] = ready[index] || healthy(*port);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        self.ready = services
            .iter()
            .zip(ready)
            .map(|((name, _), ready)| ((*name).to_string(), ready))
            .collect();
        Ok(())
    }
}
