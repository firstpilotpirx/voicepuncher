//! The supervisor: it opens the gates, runs the pipeline and reloads it.

use super::{
    lock, log_rejection, mpsc, tell, ActivityRegister, Arc, Command, Config, Context,
    HotkeyBinding, HotkeyListener, Instant, LocalServices, ModelsGate, Mutex, Next, Observers,
    PipelineActivity, PipelineAssembler, PttSignal, Receiver, ReloadListener, ReloadReport, Result,
    Running, RuntimeState, Sender, SignalRouter, Verdict, Watched, IDLE_POLL_INTERVAL, MODELS_POLL,
    RESTART_WINDOW, SERVICES_POLL,
};

/// What a restart that waits for the dictation in progress learns meanwhile.
#[derive(PartialEq, Eq)]
pub(super) enum Wait {
    Idle,
    Stopped,
}

/// What must hold before a pipeline starts: the permissions, the local services and the models.
pub(crate) struct Gates {
    pub(super) permissions: Option<Watched>,
    pub(super) services: Option<Box<dyn LocalServices>>,
    pub(super) models: Option<Box<dyn ModelsGate>>,
    /// The services were told what the configuration needs; not while models are missing.
    pub(super) services_required: bool,
}

impl Gates {
    pub(crate) fn new(
        permissions: Option<Watched>,
        services: Option<Box<dyn LocalServices>>,
        models: Option<Box<dyn ModelsGate>>,
    ) -> Self {
        Self { permissions, services, models, services_required: false }
    }
}

/// How the pipelines fared: whether one failed to start, where they report a failure and
/// when the last one was started again after one.
#[derive(Default)]
pub(super) struct PipelineHealth {
    /// A pipeline that failed to start is not started again by a permission check.
    pub(super) failed: bool,
    /// Where a pipeline reports that it failed; none in tests that build no runtime.
    pub(super) failures: Option<Sender<Command>>,
    /// Numbers the pipelines, so a failure reported by a replaced one is ignored.
    pub(super) generation: u64,
    /// When the pipeline was last started again after a failure.
    pub(super) restarted_at: Option<Instant>,
}

/// The runtime thread: every reload and the stop run here, one after the other.
pub(super) struct Supervisor {
    assembler: Box<dyn PipelineAssembler>,
    /// Spawned with the first pipeline, so it needs Input Monitoring only from then on.
    listener: Option<Box<dyn HotkeyListener>>,
    gates: Gates,
    health: PipelineHealth,
    observers: Arc<Mutex<Observers>>,
    router: SignalRouter,
    binding: HotkeyBinding,
    activity: ActivityRegister,
    pipeline: Option<Running>,
    config: Config,
}

impl Supervisor {
    pub(crate) fn new(
        assembler: Box<dyn PipelineAssembler>,
        listener: Box<dyn HotkeyListener>,
        observers: Arc<Mutex<Observers>>,
        config: Config,
        gates: Gates,
    ) -> Self {
        let mut supervisor = Self {
            assembler,
            listener: Some(listener),
            gates,
            health: PipelineHealth::default(),
            observers,
            router: SignalRouter::new(),
            binding: HotkeyBinding::new(config.hotkey.key),
            activity: ActivityRegister::default(),
            pipeline: None,
            config,
        };
        supervisor.models_missing();
        supervisor
    }

    pub(crate) fn reporting_failures_to(mut self, failures: Sender<Command>) -> Self {
        self.health.failures = Some(failures);
        self
    }

    /// Runs `next` as the current pipeline, under a new generation.
    fn launch(&mut self, next: Next) -> Result<Running> {
        self.health.generation += 1;
        next.launch(
            &self.activity,
            self.health.failures.clone(),
            self.health.generation,
        )
    }

    /// Starts a new pipeline after the current one failed; one that failed soon after such a
    /// restart fails the runtime instead.
    fn pipeline_failed(&mut self, generation: u64, reason: &str) {
        if generation != self.health.generation {
            return;
        }
        self.router.detach();
        if let Some(running) = self.pipeline.take() {
            running.join();
        }
        let soon_again = self
            .health
            .restarted_at
            .is_some_and(|at| at.elapsed() < RESTART_WINDOW);
        if soon_again {
            self.health.failed = true;
            let failure = format!("the dictation pipeline stopped: {reason}");
            self.change(RuntimeState::Failed(failure));
            return;
        }
        log::warn!("runtime: starting a new pipeline after: {reason}");
        self.health.restarted_at = Some(Instant::now());
        self.start_pipeline();
    }

    /// Starts the pipeline; a start that fails leaves the runtime Failed.
    fn start_pipeline(&mut self) {
        self.change(RuntimeState::Starting);
        match self.open() {
            Ok(()) => self.change(self.ready()),
            Err(e) => {
                self.health.failed = true;
                self.change(RuntimeState::Failed(format!("{e:#}")));
            }
        }
    }

    /// The next command; none when the permissions or the services are due for a check first.
    fn next_command(&self, commands: &Receiver<Command>) -> Option<Command> {
        let pipeline_runs = self.pipeline.is_some();
        let permissions = self
            .gates
            .permissions
            .as_ref()
            .map(|w| w.interval(pipeline_runs));
        let services = (self.gates.services.is_some() && !pipeline_runs).then_some(SERVICES_POLL);
        let models = (self.gates.models.is_some() && !pipeline_runs).then_some(MODELS_POLL);
        let Some(interval) = permissions.into_iter().chain(services).chain(models).min() else {
            return Some(commands.recv().unwrap_or(Command::Stop));
        };
        match commands.recv_timeout(interval) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => Some(Command::Stop),
        }
    }

    pub(crate) fn serve(mut self, commands: &Receiver<Command>) {
        let mut pending = std::collections::VecDeque::new();
        loop {
            let next = pending.pop_front().or_else(|| self.next_command(commands));
            let Some(command) = next else {
                self.check_gates();
                continue;
            };
            let stopped = match command {
                Command::Reload(config, mut listener) => {
                    let reload = self.reload(*config, &mut listener, commands, &mut pending);
                    reload == Wait::Stopped
                }
                Command::Stop => true,
                Command::PipelineFailed { generation, reason } => {
                    self.pipeline_failed(generation, &reason);
                    false
                }
            };
            if stopped {
                return self.stop();
            }
        }
    }

    fn stop(mut self) {
        self.router.detach();
        if let Some(running) = self.pipeline.take() {
            running.shut_down();
        }
        if let Some(services) = &mut self.gates.services {
            services.stop();
        }
        lock(&self.observers).change(RuntimeState::Stopped);
    }

    pub(crate) fn ready(&self) -> RuntimeState {
        RuntimeState::Ready {
            hotkey: self.config.hotkey.key,
            recording_mode: self.config.hotkey.recording_mode,
            voice_control: self.config.voice_control.enabled,
        }
    }

    fn change(&self, state: RuntimeState) {
        lock(&self.observers).change(state);
    }

    /// Tells the observers of `state` unless it is the current one.
    fn change_to_new(&self, state: RuntimeState) {
        let mut observers = lock(&self.observers);
        if observers.current() != Some(&state) {
            observers.change(state);
        }
    }
}

mod gates;
mod reload;
