//! Steps of `local_services.feature`: the real `just` service recipes, on free ports of the
//! scenario's own, so the shared services on 8080–8082 are never touched.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};
use cucumber::{given, then, when};

use crate::steps::off_thread;
use crate::E2eWorld;

const SPEECH_RECOGNITION: &str = "whisper-server";
const RESTART_WITHIN: Duration = Duration::from_secs(30);
const STAYS_STOPPED_FOR: Duration = Duration::from_secs(5);
const PROCESS_GONE_WITHIN: Duration = Duration::from_secs(10);
const START_FAILURE_WITHIN: Duration = Duration::from_secs(15);
/// What the supervisor writes to a service's log when the service exited and starts again.
const RESTART_NOTE: &str = "restarting in";
/// Output whisper-server writes while it loads, before any request.
const SERVICE_STARTUP_OUTPUT: &str = "whisper_init";
/// What whisper-server logs when its model file is not a model.
const BROKEN_MODEL_LOG_LINE: &str = "failed to load model";

/// The services one scenario starts through the service scripts, on ports of its own.
/// Dropping it stops them and removes their PID and log files, also when a step failed.
#[derive(Debug)]
pub struct ScenarioServices {
    pub stt_port: u16,
    llm_port: u16,
    kws_port: u16,
    speech_model: Option<PathBuf>,
    _model_dir: Option<tempfile::TempDir>,
    started: Option<Output>,
    start_took: Option<Duration>,
    supervisor: Option<u32>,
    service: Option<u32>,
    pub heard_words: Option<std::result::Result<usize, String>>,
}

impl ScenarioServices {
    fn on_free_ports() -> Result<Self> {
        Ok(Self {
            stt_port: free_port()?,
            llm_port: free_port()?,
            kws_port: free_port()?,
            speech_model: None,
            _model_dir: None,
            started: None,
            start_took: None,
            supervisor: None,
            service: None,
            heard_words: None,
        })
    }

    pub fn transcription_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.stt_port)
    }

    fn just(&self, args: &[&str]) -> Result<Output> {
        let mut just = Command::new("just");
        just.current_dir(workspace_root())
            .args(args)
            .env("VOICEPUNCHER_STT_PORT", self.stt_port.to_string())
            .env("VOICEPUNCHER_LLM_PORT", self.llm_port.to_string())
            .env("VOICEPUNCHER_KWS_PORT", self.kws_port.to_string());
        if let Some(model) = &self.speech_model {
            just.env("VOICEPUNCHER_WHISPER_MODEL", model);
        }
        just.output().context("just runs")
    }

    fn start_speech_recognition(&mut self) -> Result<()> {
        let began = Instant::now();
        let output = self.just(&["start-model-servers", SPEECH_RECOGNITION])?;
        self.start_took = Some(began.elapsed());
        if output.status.success() {
            self.supervisor = std::fs::read_to_string(self.pid_file())
                .ok()
                .and_then(|pid| pid.trim().parse().ok());
            self.service = service_process(self.stt_port);
        }
        self.started = Some(output);
        Ok(())
    }

    fn pid_file(&self) -> PathBuf {
        services_dir("run").join(format!("{SPEECH_RECOGNITION}-{}.pid", self.stt_port))
    }

    fn log_file(&self) -> PathBuf {
        services_dir("logs").join(format!("{SPEECH_RECOGNITION}-{}.log", self.stt_port))
    }
}

impl Drop for ScenarioServices {
    fn drop(&mut self) {
        let _ = self.just(&["stop-model-servers"]);
        for pid in [
            self.supervisor,
            self.service,
            service_process(self.stt_port),
        ]
        .into_iter()
        .flatten()
        {
            let _ = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .stderr(std::process::Stdio::null())
                .status();
        }
        for (dir, ext) in [("run", "pid"), ("logs", "log")] {
            for (binary, port) in [
                (SPEECH_RECOGNITION, self.stt_port),
                ("llama-server", self.llm_port),
                ("voicepuncher-command-spotting", self.kws_port),
            ] {
                let _ =
                    std::fs::remove_file(services_dir(dir).join(format!("{binary}-{port}.{ext}")));
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("e2e-test lives in the workspace")
        .to_path_buf()
}

fn services_dir(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME is set"))
        .join(".voicepuncher")
        .join(name)
}

fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn answers(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(500),
    )
    .is_ok()
}

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A process of the speech recognition service on `port`: the service itself or what keeps it
/// running, found by its command line.
fn any_process(port: u16) -> Option<u32> {
    pgrep(&format!("{SPEECH_RECOGNITION}.*--port {port}( |$)"))
}

/// The speech recognition service process itself on `port`.
fn service_process(port: u16) -> Option<u32> {
    pgrep(&format!("^{SPEECH_RECOGNITION} .*--port {port}( |$)"))
}

fn pgrep(pattern: &str) -> Option<u32> {
    let output = Command::new("pgrep").args(["-f", pattern]).output().ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .find(|pid| *pid != std::process::id())
}

fn wait_until(limit: Duration, condition: impl Fn() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    condition()
}

fn services(world: &mut E2eWorld) -> Result<&mut ScenarioServices> {
    world
        .local_services
        .as_mut()
        .context("no services were set up by a Given step")
}

#[given("no local service runs on the chosen ports")]
fn free_ports(world: &mut E2eWorld) -> Result<()> {
    world.local_services = Some(ScenarioServices::on_free_ports()?);
    Ok(())
}

#[given("the service scripts started the speech recognition service")]
async fn speech_recognition_started(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::on_free_ports()?;
        services.start_speech_recognition()?;
        let started = services.started.as_ref().expect("just started it");
        if !started.status.success() {
            bail!(
                "the service scripts could not start the speech recognition service:\n{}{}",
                String::from_utf8_lossy(&started.stdout),
                String::from_utf8_lossy(&started.stderr)
            );
        }
        world.local_services = Some(services);
        Ok(())
    })
    .await
}

#[given("the speech recognition model file is not a model")]
fn broken_speech_model(world: &mut E2eWorld) -> Result<()> {
    let mut services = ScenarioServices::on_free_ports()?;
    let dir = tempfile::tempdir()?;
    let model = dir.path().join("ggml-broken.bin");
    std::fs::write(&model, [0x5a_u8; 4096])?;
    services.speech_model = Some(model);
    services._model_dir = Some(dir);
    world.local_services = Some(services);
    Ok(())
}

#[when(
    regex = r"^(?:the user starts only the speech recognition service|the service scripts start the speech recognition service)(?: with the service scripts)?$"
)]
async fn start_speech_recognition(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| services(world)?.start_speech_recognition()).await
}

#[when("its process exits on its own")]
async fn service_dies(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let pid = services
            .service
            .context("no speech recognition service process was found after its start")?;
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;
        wait_until(PROCESS_GONE_WITHIN, || !alive(pid));
        Ok(())
    })
    .await
}

#[when("the user stops it with the service scripts")]
async fn user_stops_service(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        services(world)?.just(&["stop-model-servers", SPEECH_RECOGNITION])?;
        Ok(())
    })
    .await
}

#[then("the speech recognition service answers and nothing answers on the text refinement and command spotting ports")]
fn only_speech_recognition_answers(world: &mut E2eWorld) -> Result<()> {
    let services = services(world)?;
    let started = services.started.as_ref().context("nothing was started")?;
    ensure!(
        started.status.success(),
        "starting only the speech recognition service failed:\n{}{}",
        String::from_utf8_lossy(&started.stdout),
        String::from_utf8_lossy(&started.stderr)
    );
    ensure!(
        answers(services.stt_port),
        "the speech recognition service does not answer"
    );
    ensure!(
        !answers(services.llm_port),
        "a text refinement service was started too"
    );
    ensure!(
        !answers(services.kws_port),
        "a command spotting service was started too"
    );
    Ok(())
}

#[then("the service answers on its port again within 30 seconds, and its log keeps the output of the run that ended followed by a note of the restart")]
async fn service_restarted(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let port = services.stt_port;
        ensure!(
            wait_until(RESTART_WITHIN, || answers(port)),
            "the speech recognition service did not answer within {RESTART_WITHIN:?} after its process exited"
        );
        let log = std::fs::read_to_string(services.log_file())
            .with_context(|| format!("log {}", services.log_file().display()))?;
        let note = log
            .find(RESTART_NOTE)
            .context("the log has no note of the restart")?;
        ensure!(
            log[..note].contains(SERVICE_STARTUP_OUTPUT),
            "the log lost the output of the run that ended"
        );
        Ok(())
    })
    .await
}

#[then("nothing answers on its port, still 5 seconds later, and no process of it is left")]
async fn service_stays_stopped(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let port = services.stt_port;
        ensure!(
            !answers(port),
            "the service still answers after it was stopped"
        );
        std::thread::sleep(STAYS_STOPPED_FOR);
        ensure!(
            !answers(port),
            "the service answers again {STAYS_STOPPED_FOR:?} after it was stopped"
        );
        ensure!(
            any_process(port).is_none(),
            "a process of the stopped service is left: {:?}",
            any_process(port)
        );
        Ok(())
    })
    .await
}

#[then("the start fails within 15 seconds naming the service and its log, and no process of it is left")]
async fn start_failed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = services(world)?;
        let started = services.started.as_ref().context("nothing was started")?;
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&started.stdout),
            String::from_utf8_lossy(&started.stderr)
        );
        ensure!(
            !started.status.success(),
            "the start of a service with a broken model succeeded:\n{said}"
        );
        let took = services.start_took.context("the start was not timed")?;
        ensure!(
            took <= START_FAILURE_WITHIN,
            "the start failed only after {took:?}, not within {START_FAILURE_WITHIN:?}"
        );
        let log = services.log_file();
        let report = String::from_utf8_lossy(&started.stderr);
        ensure!(
            report.contains(SPEECH_RECOGNITION) && report.contains(&*log.to_string_lossy()),
            "the failure report does not name {SPEECH_RECOGNITION} and its log {}:\n{report}",
            log.display()
        );
        ensure!(
            report.contains(BROKEN_MODEL_LOG_LINE),
            "the failure report does not show the service's last log lines:\n{report}"
        );
        let port = services.stt_port;
        ensure!(
            wait_until(PROCESS_GONE_WITHIN, || any_process(port).is_none()),
            "a process of the service that failed to start is left: {:?}",
            any_process(port)
        );
        ensure!(
            !services.pid_file().exists(),
            "the PID file of the service that failed to start is left"
        );
        Ok(())
    })
    .await
}
