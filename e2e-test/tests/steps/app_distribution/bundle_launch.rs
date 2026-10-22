//! The `e2e-bundle` stage: a copy of the bundle the bundle stage built, launched outside the
//! build folder with nothing but the system folders on its path, so every service it starts runs
//! the bundle's own engines; then one transcription, one refinement and the command spotting
//! service's health through them, and the engines' logs read for the Metal GPU.

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, ensure, Context, Result};
use cucumber::{given, then, when};
use voicepuncher::adapters::secondary::logging::file::{logs_dir, LOG_FILE_NAME};
use voicepuncher::adapters::secondary::textrefinement::openai::OpenAiTextRefiner;
use voicepuncher::adapters::secondary::transcription::openai::OpenAiTranscriber;
use voicepuncher::config::{RefinementCfg, TranscriptionCfg};
use voicepuncher::core::application::{TextRefiner, Transcriber};
use voicepuncher::core::domain::{AudioBuffer16k, RawTranscript};
use voicepuncher_copy_pool::ServiceAddress;
use voicepuncher_e2e_test::corpus::load_manifest;
use voicepuncher_e2e_test::speech::read_wav;

use super::engines::{checkout_lock, read_lock};
use super::{copy_bundle, stage_bundle, BUNDLE_DIR};
use crate::steps::off_thread;
use crate::E2eWorld;

/// The models the services need, as the `justfile` exports them; the only variables the copy
/// gets beside its home and its path.
const MODELS: [&str; 6] = [
    "VOICEPUNCHER_WHISPER_MODEL",
    "VOICEPUNCHER_VAD_MODEL",
    "VOICEPUNCHER_COMMAND_WHISPER_MODEL",
    "VOICEPUNCHER_LLM_MODEL",
    "VOICEPUNCHER_KWS_MODEL_DIR",
    "VOICEPUNCHER_KWS_VAD_MODEL",
];
const CORPUS: &str = "VOICEPUNCHER_BENCH_CORPUS";
const APP_PROGRAM: &str = "Contents/MacOS/voicepuncher";
const ENGINES_DIR: &str = "Contents/MacOS/engines";
/// The system folders a Mac without Homebrew has on its path.
const SYSTEM_PATH: &str = "/usr/bin:/bin";
/// The engine sections of the lock, by the program each runs.
const ENGINES: [&str; 3] = ["whisper", "llama", "command-spotting"];
/// What the app logs for an engine it takes from the bundle.
const FROM_BUNDLE: &str = "from the bundle";
/// What whisper-server logs when it sets up the Metal device and its library; a line of them
/// that says error or failed is a Metal failure.
const WHISPER_METAL: &str = "ggml_metal_library_init: using";
const METAL_PREFIX: &str = "ggml_metal";
const METAL_FAILURES: [&str; 2] = ["error", "failed"];
/// What ggml logs for each backend it loads, followed by the file it loaded.
const BACKEND_LOADED: &str = "load_backend: loaded";
/// llama-server logs no device at its default verbosity: the copy's own llama-server lists the
/// devices its backends found, and the Metal one is named `MTL0`.
const LLAMA_DEVICES: &str = "--list-devices";
const LLAMA_METAL: &str = "MTL0";
const UNPUNCTUATED: &str = "я думаю что это хорошая идея но надо обсудить с командой";
/// Loading the dictation and refinement models takes 5–30 s; the app gives each start 60 s.
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
    llm: u16,
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

fn program(engine: &str) -> Result<String> {
    read_lock(&checkout_lock())?
        .get(engine)
        .and_then(|table| table.get("program"))
        .cloned()
        .with_context(|| format!("the lock names no program for {engine}"))
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
        llm: free_port()?,
        kws: free_port()?,
    };
    Ok(())
}

#[when("the copy's application starts its services with refinement and voice control on")]
#[expect(clippy::too_many_lines, reason = "size baseline")]
async fn copy_starts_services(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let launch = launch(world);
        let Ports { stt, command_stt, llm, kws } = launch.ports;
        let app = launch
            .folder
            .as_ref()
            .context("no copy")?
            .path()
            .join(BUNDLE_DIR)
            .join(APP_PROGRAM);
        let home = tempfile::tempdir()?;
        let config = home.path().join("config.toml");
        // One copy of each service, so each answers on the port the scenario checks
        fs::write(
            &config,
            format!(
                "[transcription]\nbase_url = \"{}\"\n\n\
                 [refinement]\nenabled = true\nbase_url = \"{}\"\n\n\
                 [voice_control]\nenabled = true\nurl = \"ws://127.0.0.1:{kws}\"\n\n\
                 [services]\nmanage = true\ndictation_copies = 1\ncommand_copies = 1\n\
                 refinement_copies = 1\ncommand_spotting_copies = 1\n\
                 commands_url = \"{}\"\n",
                url(stt),
                url(llm),
                url(command_stt)
            ),
        )?;
        // Nothing of the developer's environment: no Homebrew on the path, no *_BINARY, no DYLD_*
        let mut command = Command::new(&app);
        command
            .arg("--config")
            .arg(&config)
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", SYSTEM_PATH)
            .current_dir(home.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        for variable in MODELS {
            command.env(variable, std::env::var(variable)?);
        }
        launch.child = Some(command.spawn().context("the copy's app could not start")?);
        launch.home = Some(home);
        let services = [
            ("speech recognition", stt),
            ("speech recognition for commands", command_stt),
            ("text refinement", llm),
            ("command spotting", kws),
        ];
        let deadline = Instant::now() + READY_WITHIN;
        let mut ready = vec![false; services.len()];
        while Instant::now() < deadline && ready.contains(&false) {
            if let Some(status) = launch.child.as_mut().context("no app")?.try_wait()? {
                bail!(
                    "the copy's app ended with {status}; its log:\n{}",
                    launch.app_log()
                );
            }
            for (index, (_, port)) in services.iter().enumerate() {
                ready[index] = ready[index] || healthy(*port);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        launch.ready = services
            .iter()
            .zip(ready)
            .map(|((name, _), ready)| (name.to_string(), ready))
            .collect();
        Ok(())
    })
    .await
}

fn all_ready(launch: &BundleLaunch) -> Result<()> {
    let late: Vec<&str> = launch
        .ready
        .iter()
        .filter(|(_, ready)| !ready)
        .map(|(name, _)| name.as_str())
        .collect();
    ensure!(
        late.is_empty() && !launch.ready.is_empty(),
        "not ready within {READY_WITHIN:?}: {late:?}; the app's log:\n{}",
        launch.app_log()
    );
    Ok(())
}

#[then("its log names each engine as taken from the bundle")]
fn engines_from_bundle(world: &mut E2eWorld) -> Result<()> {
    let launch = launch(world);
    all_ready(launch)?;
    let log = launch.app_log();
    for engine in ENGINES {
        let program = program(engine)?;
        ensure!(
            log.lines().any(
                |line| line.contains(&format!("runs {program} ")) && line.contains(FROM_BUNDLE)
            ),
            "the app's log does not name {program} {FROM_BUNDLE}:\n{log}"
        );
    }
    Ok(())
}

#[then("a recording is transcribed and a sentence is refined through the started services")]
async fn transcribe_and_refine(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let ports = launch(world).ports;
        let corpus = PathBuf::from(std::env::var(CORPUS)?);
        let recording = load_manifest(&corpus.join("manifest.jsonl"))?
            .into_iter()
            .next()
            .context("the corpus is empty")?;
        let mut transcriber = OpenAiTranscriber::connect(TranscriptionCfg {
            base_url: url(ports.stt),
            ..TranscriptionCfg::default()
        })?;
        let transcript = transcriber
            .transcribe(&AudioBuffer16k::new(read_wav(&recording.path)?))?
            .context("the bundled speech recognition engine heard nothing")?;
        ensure!(
            !transcript.as_str().trim().is_empty(),
            "an empty transcript of {}",
            recording.path.display()
        );
        let mut refiner = OpenAiTextRefiner::connect(RefinementCfg {
            base_url: url(ports.llm),
            ..RefinementCfg::default()
        })?;
        let refined = refiner.refine(&RawTranscript::new(UNPUNCTUATED.to_string())?)?;
        ensure!(
            !refined.as_str().trim().is_empty(),
            "the bundled text refinement engine answered nothing"
        );
        Ok(())
    })
    .await
}

#[then("the command spotting service answers its health check")]
fn spotting_healthy(world: &mut E2eWorld) -> Result<()> {
    let port = launch(world).ports.kws;
    ensure!(
        healthy(port),
        "the command spotting service on port {port} does not answer its health check"
    );
    Ok(())
}

#[then(
    "the speech recognition engine's log shows it loaded its compute backends from the copy and set up the Metal GPU"
)]
fn whisper_on_metal(world: &mut E2eWorld) -> Result<()> {
    let launch = launch(world);
    let copy = launch
        .folder
        .as_ref()
        .context("no copy")?
        .path()
        .to_path_buf();
    let copy = copy.canonicalize().unwrap_or(copy);
    let whisper = program("whisper")?;
    let log = launch.engine_log(&whisper, launch.ports.stt)?;
    let loaded: Vec<&str> = log
        .lines()
        .filter(|line| line.contains(BACKEND_LOADED))
        .collect();
    ensure!(
        !loaded.is_empty(),
        "{whisper} logged no backend it loaded:\n{log}"
    );
    for line in &loaded {
        let from = line.rsplit(' ').next().unwrap_or_default();
        let from = Path::new(from)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(from));
        ensure!(
            from.starts_with(&copy),
            "{whisper} loaded a backend from outside the copy: {line}"
        );
    }
    ensure!(
        log.contains(WHISPER_METAL),
        "{whisper}'s log does not show the Metal GPU ({WHISPER_METAL}):\n{log}"
    );
    if let Some(failure) = log.lines().find(|line| {
        line.contains(METAL_PREFIX) && METAL_FAILURES.iter().any(|word| line.contains(word))
    }) {
        bail!("{whisper} failed on Metal: {failure}");
    }
    Ok(())
}

#[then("the copy's text refinement engine finds the Metal GPU")]
fn llama_finds_metal(world: &mut E2eWorld) -> Result<()> {
    let launch = launch(world);
    let llama = program("llama")?;
    let path = launch
        .folder
        .as_ref()
        .context("no copy")?
        .path()
        .join(BUNDLE_DIR)
        .join(ENGINES_DIR)
        .join("llama")
        .join(&llama);
    let output = Command::new(&path)
        .arg(LLAMA_DEVICES)
        .env_clear()
        .env("PATH", SYSTEM_PATH)
        .output()
        .with_context(|| format!("{} could not start", path.display()))?;
    let devices = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    ensure!(
        output.status.success() && devices.contains(LLAMA_METAL),
        "the bundle's {llama} finds no Metal device ({LLAMA_METAL}):\n{devices}"
    );
    Ok(())
}
