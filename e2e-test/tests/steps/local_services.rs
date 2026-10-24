//! Steps of `local_services.feature`: the real `just` service recipes, on free ports of the
//! scenario's own, so the shared services on 8080–8082 are never touched.

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, ensure, Context, Result};
use cucumber::{given, then, when};
use voicepuncher_copy_pool::ServiceAddress;

use crate::{steps::off_thread, E2eWorld};

mod app;
mod confirmations;
mod copies;
mod hangs;
mod managed;
mod models;
mod owned;
mod warm_up;

pub use managed::{capture_log, log_mark, logged_since, ManagedRun, Needed};
pub use models::Role;
pub use owned::OwnedServices;

const SPEECH_RECOGNITION: &str = "whisper-server";
/// The speech recognition service for commands, by its name in the service scripts.
const COMMAND_SPEECH_RECOGNITION: &str = "whisper-server-commands";
const COMMAND_SPOTTING: &str = "voicepuncher-command-spotting";
/// How long a health check may take before it counts as failed.
const HEALTH_CHECK_WAIT: Duration = Duration::from_secs(1);
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
    /// The port of the speech recognition service for commands, passed on every call of the
    /// service scripts, so they never stop or reuse the shared one.
    command_stt_port: u16,
    kws_port: u16,
    speech_model: Option<PathBuf>,
    /// Whether the dictation and commands model variables are removed from the scripts'
    /// environment, so the scripts pick their own default models.
    default_speech_model: bool,
    default_command_model: bool,
    /// A home of the scenario's own, holding the models the scripts download; the service
    /// scripts run in it (models, logs, PID files) when set.
    home: Option<tempfile::TempDir>,
    /// What each start of a single service by name printed, in start order.
    starts: Vec<(String, Output)>,
    _model_dir: Option<tempfile::TempDir>,
    started: Option<Output>,
    start_took: Option<Duration>,
    supervisor: Option<u32>,
    service: Option<u32>,
    pub heard_words: Option<std::result::Result<usize, String>>,
    /// How many copies of the speech recognition service to start; the scripts' default
    /// when `None`.
    copies: Option<u32>,
    /// How many copies of the speech recognition service for commands to start; the
    /// scripts' default when `None`.
    command_copies: Option<u32>,
    /// The ports of the copies the start left running.
    copy_ports: Vec<u16>,
    /// How many seconds apart the scripts check that a service still answers; the scripts'
    /// default when `None`.
    health_check_secs: Option<u32>,
    pool: copies::PoolRun,
    /// Processes a step paused: they are resumed before the services are stopped.
    paused: Vec<u32>,
    /// The command spotting service process a step made hang or watched.
    hung: Option<u32>,
    /// How long the command spotting service's log was before a step streamed speech.
    log_before_speech: Option<u64>,
    /// The app that started its services itself.
    app: Option<app::LaunchedApp>,
}

impl ScenarioServices {
    /// Services on free ports, the speech recognition service as one copy, as in the
    /// scenarios about a single service.
    fn on_free_ports() -> Result<Self> {
        Self::with_copies(Some(1))
    }

    fn with_copies(copies: Option<u32>) -> Result<Self> {
        Ok(Self {
            stt_port: free_speech_recognition_port()?,
            command_stt_port: free_speech_recognition_port()?,
            kws_port: free_port()?,
            speech_model: None,
            default_speech_model: false,
            default_command_model: false,
            home: None,
            starts: Vec::new(),
            _model_dir: None,
            started: None,
            start_took: None,
            supervisor: None,
            service: None,
            heard_words: None,
            copies,
            command_copies: None,
            copy_ports: Vec::new(),
            health_check_secs: None,
            pool: copies::PoolRun::default(),
            paused: Vec::new(),
            hung: None,
            log_before_speech: None,
            app: None,
        })
    }

    /// Services on free ports, the speech recognition service for commands as one copy, for
    /// the voice control scenarios that make them fail: a paused copy then stalls every
    /// confirmation, as no other copy routes around it.
    /// Makes the scripts check every `secs` seconds that a service still answers, when they
    /// start it next.
    pub fn check_health_every(&mut self, secs: u32) {
        self.health_check_secs = Some(secs);
    }

    pub fn for_outages() -> Result<Self> {
        let mut services = Self::on_free_ports()?;
        services.command_copies = Some(1);
        Ok(services)
    }

    /// The speech recognition service for commands started by the service scripts as one
    /// copy, on free ports.
    pub fn with_command_service() -> Result<Self> {
        let mut services = Self::for_outages()?;
        services.start(&[COMMAND_SPEECH_RECOGNITION])?;
        Ok(services)
    }

    /// Starts the named services with the service scripts; fails with their output when the
    /// start fails.
    pub fn start(&mut self, names: &[&str]) -> Result<()> {
        let args: Vec<&str> = std::iter::once("start-model-servers")
            .chain(names.iter().copied())
            .collect();
        let output = self.just(&args)?;
        ensure!(
            output.status.success(),
            "the service scripts could not start {names:?}:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        self.copy_ports = self.running_copy_ports();
        self.service = service_process(self.stt_port);
        Ok(())
    }

    pub fn command_spotting_url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.kws_port)
    }

    pub fn kws_port(&self) -> u16 {
        self.kws_port
    }

    /// The command spotting service process itself, not what keeps it running.
    pub fn command_spotting_process(&self) -> Option<u32> {
        pgrep(&format!(
            "^[^ ]*{COMMAND_SPOTTING} --port {}( |$)",
            self.kws_port
        ))
    }

    /// What keeps the command spotting service running: the supervisor in its PID file.
    pub fn command_spotting_supervisor(&self) -> Option<u32> {
        std::fs::read_to_string(
            self.dir("run")
                .join(format!("{COMMAND_SPOTTING}-{}.pid", self.kws_port)),
        )
        .ok()?
        .trim()
        .parse()
        .ok()
    }

    /// The speech recognition service for commands, run as one copy: its process itself.
    pub fn command_speech_recognition_process(&self) -> Option<u32> {
        service_process(self.command_stt_port)
    }

    pub fn command_spotting_log(&self) -> PathBuf {
        self.dir("logs")
            .join(format!("{COMMAND_SPOTTING}-{}.log", self.kws_port))
    }

    /// A directory of the service scripts (`run`, `logs`), in the scenario's home when it
    /// has one.
    fn dir(&self, name: &str) -> PathBuf {
        match &self.home {
            Some(home) => home.path().join(".voicepuncher").join(name),
            None => services_dir(name),
        }
    }

    /// Stops `pid` without ending it, as a process that hangs; resumed when the services stop.
    pub fn pause(&mut self, pid: u32) -> Result<()> {
        signal(pid, "-STOP")?;
        self.paused.push(pid);
        Ok(())
    }

    pub fn resume(&mut self, pid: u32) -> Result<()> {
        self.paused.retain(|p| *p != pid);
        signal(pid, "-CONT")
    }

    pub fn transcription_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.stt_port)
    }

    /// The address of the speech recognition service for commands.
    pub fn command_transcription_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.command_stt_port)
    }

    fn just(&self, args: &[&str]) -> Result<Output> {
        let mut just = Command::new("just");
        just.current_dir(workspace_root())
            .args(args)
            .env("VOICEPUNCHER_STT_PORT", self.stt_port.to_string())
            .env(
                "VOICEPUNCHER_COMMAND_STT_PORT",
                self.command_stt_port.to_string(),
            )
            .env("VOICEPUNCHER_KWS_PORT", self.kws_port.to_string());
        if let Some(home) = &self.home {
            just.env("HOME", home.path());
        }
        if let Some(secs) = self.health_check_secs {
            just.env("VOICEPUNCHER_HEALTH_CHECK_SECS", secs.to_string());
        }
        if let Some(copies) = self.copies {
            just.env("VOICEPUNCHER_STT_INSTANCES", copies.to_string());
        }
        if let Some(copies) = self.command_copies {
            just.env("VOICEPUNCHER_COMMAND_STT_INSTANCES", copies.to_string());
        }
        if let Some(model) = &self.speech_model {
            just.env("VOICEPUNCHER_WHISPER_MODEL", model);
        } else if self.default_speech_model {
            just.env_remove("VOICEPUNCHER_WHISPER_MODEL");
        }
        if self.default_command_model {
            just.env_remove("VOICEPUNCHER_COMMAND_WHISPER_MODEL");
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
            self.copy_ports = self.running_copy_ports();
        }
        self.started = Some(output);
        Ok(())
    }

    /// Points the speech recognition service at a file that is not a model.
    fn break_speech_model(&mut self) -> Result<()> {
        let dir = tempfile::tempdir()?;
        let model = dir.path().join("ggml-broken.bin");
        std::fs::write(&model, [0x5a_u8; 4096])?;
        self.speech_model = Some(model);
        self._model_dir = Some(dir);
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
    #[expect(clippy::too_many_lines, reason = "size baseline")]
    fn drop(&mut self) {
        for pid in std::mem::take(&mut self.paused) {
            let _ = signal(pid, "-CONT");
        }
        let left_spotting = self.command_spotting_process();
        let command_copy_ports = self.copy_ports_of(self.command_stt_port);
        let _ = self.just(&["stop-model-servers"]);
        let left = self
            .copy_ports
            .iter()
            .chain(&command_copy_ports)
            .filter_map(|port| any_process(*port));
        for pid in [
            self.supervisor,
            self.service,
            service_process(self.stt_port),
            any_process(self.command_stt_port),
            left_spotting,
            self.command_spotting_process(),
        ]
        .into_iter()
        .flatten()
        .chain(left)
        {
            let _ = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .stderr(std::process::Stdio::null())
                .status();
        }
        for (dir, ext) in [("run", "pid"), ("logs", "log")] {
            for (binary, port) in [
                (SPEECH_RECOGNITION, self.stt_port),
                (SPEECH_RECOGNITION, self.command_stt_port),
                ("voicepuncher-command-spotting", self.kws_port),
            ] {
                let _ =
                    std::fs::remove_file(services_dir(dir).join(format!("{binary}-{port}.{ext}")));
            }
            for port in self.copy_ports.iter().chain(&command_copy_ports) {
                let _ = std::fs::remove_file(
                    services_dir(dir).join(format!("{SPEECH_RECOGNITION}-{port}.{ext}")),
                );
            }
        }
        copies::remove_copy_list(self.stt_port);
        copies::remove_copy_list(self.command_stt_port);
    }
}

fn signal(pid: u32, signal: &str) -> Result<()> {
    let status = Command::new("kill")
        .args([signal, &pid.to_string()])
        .status()?;
    ensure!(status.success(), "kill {signal} {pid} failed");
    Ok(())
}

/// The service on `port` answers its health check.
pub fn healthy(port: u16) -> bool {
    ureq::get(&format!("http://127.0.0.1:{port}/health"))
        .timeout(HEALTH_CHECK_WAIT)
        .call()
        .is_ok()
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

/// A free port for a speech recognition service whose copy ports are free too: the service
/// scripts and the clients take anything listening on them for a copy of the service.
fn free_speech_recognition_port() -> Result<u16> {
    loop {
        let port = free_port()?;
        let copy_ports = ServiceAddress::parse(&format!("http://127.0.0.1:{port}/v1"))
            .map_err(|e| anyhow!("{e}"))?
            .copy_ports();
        if !copy_ports.into_iter().any(answers) {
            return Ok(port);
        }
    }
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
    services.break_speech_model()?;
    world.local_services = Some(services);
    Ok(())
}

#[when(
    regex = r"^(?:the user starts only the speech recognition service for dictation|the service scripts start the speech recognition service)(?: with the service scripts)?$"
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

#[then("the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands and the command spotting service")]
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
        "the speech recognition service for dictation does not answer"
    );
    ensure!(
        !answers(services.command_stt_port),
        "a speech recognition service for commands was started too"
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
#[expect(clippy::too_many_lines, reason = "size baseline")]
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
