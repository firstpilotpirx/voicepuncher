//! Steps of `local_services.feature`: the real `just` service recipes, on free ports of the
//! scenario's own, so the shared services on 8080–8082 are never touched.

use std::net::{SocketAddr, TcpListener, TcpStream};
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use voicepuncher_copy_pool::ServiceAddress;

use crate::{steps::off_thread, E2eWorld};

mod app;
mod confirmations;
mod copies;
mod hangs;
mod managed;
mod model_home;
mod models;
mod owned;
mod warm_up;

pub use managed::{
    capture_log, connected_copy, kill_copy, log_mark, logged_since, ManagedRun, Needed,
};
pub use models::Role;
pub use owned::OwnedServices;

mod processes;
pub use processes::*;
mod service_steps;
use service_steps::services;

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
    options: ScriptOptions,
    /// A home of the scenario's own, holding the models the scripts download; the service
    /// scripts run in it (models, logs, PID files) when set.
    home: Option<tempfile::TempDir>,
    model_dir: Option<tempfile::TempDir>,
    run: StartRun,
    pub heard_words: Option<std::result::Result<usize, String>>,
    /// The ports of the copies the start left running.
    copy_ports: Vec<u16>,
    pool: copies::PoolRun,
    watched: Watched,
    /// The app that started its services itself.
    app: Option<app::LaunchedApp>,
}

/// What the scenario tells the service scripts beside the ports.
#[derive(Debug, Default)]
struct ScriptOptions {
    speech_model: Option<PathBuf>,
    /// Whether the dictation and commands model variables are removed from the scripts'
    /// environment, so the scripts pick their own default models.
    default_speech_model: bool,
    default_command_model: bool,
    /// How many copies of the speech recognition service to start; the scripts' default
    /// when `None`.
    copies: Option<u32>,
    /// How many copies of the speech recognition service for commands to start; the
    /// scripts' default when `None`.
    command_copies: Option<u32>,
    /// How many seconds apart the scripts check that a service still answers; the scripts'
    /// default when `None`.
    health_check_secs: Option<u32>,
}

/// What the starts of the services printed and left running.
#[derive(Debug, Default)]
struct StartRun {
    /// What each start of a single service by name printed, in start order.
    starts: Vec<(String, Output)>,
    started: Option<Output>,
    start_took: Option<Duration>,
    supervisor: Option<u32>,
    service: Option<u32>,
}

/// The processes the steps paused, hung or watched.
#[derive(Debug, Default)]
struct Watched {
    /// Processes a step paused: they are resumed before the services are stopped.
    paused: Vec<u32>,
    /// The command spotting service process a step made hang or watched.
    hung: Option<u32>,
    /// How long the command spotting service's log was before a step streamed speech.
    log_before_speech: Option<u64>,
}

impl ScenarioServices {
    /// What the service scripts' start printed and how it ended.
    pub(super) fn started(&self) -> Result<&Output> {
        self.run.started.as_ref().context("nothing was started")
    }

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
            options: ScriptOptions { copies, ..ScriptOptions::default() },
            home: None,
            model_dir: None,
            run: StartRun::default(),
            heard_words: None,
            copy_ports: Vec::new(),
            pool: copies::PoolRun::default(),
            watched: Watched::default(),
            app: None,
        })
    }

    /// Services on free ports, the speech recognition service for commands as one copy, for
    /// the voice control scenarios that make them fail: a paused copy then stalls every
    /// confirmation, as no other copy routes around it.
    /// Makes the scripts check every `secs` seconds that a service still answers, when they
    /// start it next.
    pub fn check_health_every(&mut self, secs: u32) {
        self.options.health_check_secs = Some(secs);
    }

    pub fn for_outages() -> Result<Self> {
        let mut services = Self::on_free_ports()?;
        services.options.command_copies = Some(1);
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
        self.run.service = service_process(self.stt_port);
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
        self.watched.paused.push(pid);
        Ok(())
    }

    pub fn resume(&mut self, pid: u32) -> Result<()> {
        self.watched.paused.retain(|p| *p != pid);
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
        if let Some(secs) = self.options.health_check_secs {
            just.env("VOICEPUNCHER_HEALTH_CHECK_SECS", secs.to_string());
        }
        if let Some(copies) = self.options.copies {
            just.env("VOICEPUNCHER_STT_INSTANCES", copies.to_string());
        }
        if let Some(copies) = self.options.command_copies {
            just.env("VOICEPUNCHER_COMMAND_STT_INSTANCES", copies.to_string());
        }
        if let Some(model) = &self.options.speech_model {
            just.env("VOICEPUNCHER_WHISPER_MODEL", model);
        } else if self.options.default_speech_model {
            just.env_remove("VOICEPUNCHER_WHISPER_MODEL");
        }
        if self.options.default_command_model {
            just.env_remove("VOICEPUNCHER_COMMAND_WHISPER_MODEL");
        }
        just.output().context("just runs")
    }

    fn start_speech_recognition(&mut self) -> Result<()> {
        let began = Instant::now();
        let output = self.just(&["start-model-servers", SPEECH_RECOGNITION])?;
        self.run.start_took = Some(began.elapsed());
        if output.status.success() {
            self.run.supervisor = std::fs::read_to_string(self.pid_file())
                .ok()
                .and_then(|pid| pid.trim().parse().ok());
            self.run.service = service_process(self.stt_port);
            self.copy_ports = self.running_copy_ports();
        }
        self.run.started = Some(output);
        Ok(())
    }

    /// Points the speech recognition service at a file that is not a model.
    fn break_speech_model(&mut self) -> Result<()> {
        let dir = tempfile::tempdir()?;
        let model = dir.path().join("ggml-broken.bin");
        std::fs::write(&model, [0x5a_u8; 4096])?;
        self.options.speech_model = Some(model);
        self.model_dir = Some(dir);
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
        for pid in std::mem::take(&mut self.watched.paused) {
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
            self.run.supervisor,
            self.run.service,
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
        self.remove_service_files(&command_copy_ports);
        copies::remove_copy_list(self.stt_port);
        copies::remove_copy_list(self.command_stt_port);
    }
}

impl ScenarioServices {
    /// Removes the pid and log files of the scenario's services and copies.
    fn remove_service_files(&self, command_copy_ports: &[u16]) {
        for (dir, ext) in [("run", "pid"), ("logs", "log")] {
            for (binary, port) in [
                (SPEECH_RECOGNITION, self.stt_port),
                (SPEECH_RECOGNITION, self.command_stt_port),
                ("voicepuncher-command-spotting", self.kws_port),
            ] {
                let _ =
                    std::fs::remove_file(services_dir(dir).join(format!("{binary}-{port}.{ext}")));
            }
            for port in self.copy_ports.iter().chain(command_copy_ports) {
                let _ = std::fs::remove_file(
                    services_dir(dir).join(format!("{SPEECH_RECOGNITION}-{port}.{ext}")),
                );
            }
        }
    }
}
