//! The life of the application between launch and quit: the dictation pipeline starts from
//! the configuration, stops, and restarts with new settings while the process keeps running.

pub mod activity;
pub mod assembler;
pub mod models;
pub mod observer;
pub mod overlay;
pub mod permissions;
pub mod report;
pub mod router;
pub mod services;
pub mod signals;
pub mod wiring;

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::config::{self, Config};
use crate::core::application::pipeline::Pipeline;
use crate::core::application::{
    HotkeyListener, PipelineActivity, PipelineActivitySink, PipelineInput,
};
use crate::core::domain::{HotkeyBinding, Permission, PttSignal};

use permissions::{Verdict, Watched};
use services::SERVICES_POLL;

pub use activity::ActivityRegister;
pub use assembler::PipelineAssembler;
pub use models::{CatalogModels, ModelsGate};
pub use observer::{LogObserver, Observers, RuntimeObserver, RuntimeState};
pub use overlay::SharedVisualIndicator;
pub use permissions::{PermissionsWait, PermissionsWatch};
pub use report::{ReloadListener, ReloadReport};
pub use router::SignalRouter;
pub use services::{LocalServices, ManagedServices, ServicesView};

/// How often the runtime looks for the model files it waits for.
const MODELS_POLL: Duration = Duration::from_secs(1);
/// How long a restart waits for the dictation in progress between its checks for a stop, and
/// the grace it gives a signal on its way to the pipeline after the detach.
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(20);
/// A pipeline that fails this soon after the restart that followed a failure is not
/// restarted again.
const RESTART_WINDOW: Duration = Duration::from_secs(30);

/// Owns the hotkey listener for the life of the process and one pipeline at a time.
pub struct Runtime {
    assembler: Option<Box<dyn PipelineAssembler>>,
    listener: Option<Box<dyn HotkeyListener>>,
    permissions: Option<PermissionsWatch>,
    services: Option<Box<dyn LocalServices>>,
    models: Option<Box<dyn ModelsGate>>,
    observers: Arc<Mutex<Observers>>,
    commands: Option<Sender<Command>>,
    thread: Option<JoinHandle<()>>,
}

enum Command {
    Reload(Box<Config>, Option<ReloadListener>),
    Stop,
    /// The pipeline of `generation` ended with an error or a panic.
    PipelineFailed {
        generation: u64,
        reason: String,
    },
}

impl Runtime {
    pub fn new(assembler: Box<dyn PipelineAssembler>, listener: Box<dyn HotkeyListener>) -> Self {
        Self {
            assembler: Some(assembler),
            listener: Some(listener),
            permissions: None,
            services: None,
            models: None,
            observers: Arc::default(),
            commands: None,
            thread: None,
        }
    }

    /// Starts the pipeline only while every permission is granted, and stops it when one is
    /// revoked; without a watch the permissions are not checked.
    pub fn with_permissions(mut self, watch: PermissionsWatch) -> Self {
        self.permissions = Some(watch);
        self
    }

    /// Starts the local services each configuration needs and starts a pipeline only once
    /// they answer; without them the runtime only connects to the services.
    pub fn with_services(mut self, services: Box<dyn LocalServices>) -> Self {
        self.services = Some(services);
        self
    }

    /// Starts no service and no pipeline while a model file the configuration needs is
    /// missing; without a gate the models are not checked.
    pub fn with_models(mut self, models: Box<dyn ModelsGate>) -> Self {
        self.models = Some(models);
        self
    }

    /// Tells `observer` of every state change from now on, and of the current state at once.
    pub fn observe(&self, observer: Box<dyn RuntimeObserver>) {
        lock(&self.observers).register(observer);
    }

    /// The current state; none before the start.
    pub fn state(&self) -> Option<RuntimeState> {
        lock(&self.observers).current().cloned()
    }

    /// Starts the pipeline from `config` and returns once it runs, or with the reason it
    /// cannot; the state is then Ready or Failed. With a permission missing or a needed local
    /// service not ready it returns at once in the state Waiting for permissions or Starting
    /// services, and starts the pipeline once all are granted and every service answers.
    pub fn start(&mut self, config: Config) -> Result<()> {
        let (Some(assembler), Some(listener)) = (self.assembler.take(), self.listener.take())
        else {
            bail!("the runtime was started already");
        };
        let (commands_tx, commands_rx) = mpsc::channel();
        let mut supervisor = Supervisor::new(
            assembler,
            listener,
            Arc::clone(&self.observers),
            config,
            self.permissions.take().map(Watched::start),
            self.services.take(),
            self.models.take(),
        )
        .reporting_failures_to(commands_tx.clone());
        if let Some(state) = supervisor.gate() {
            self.change(state);
        } else {
            self.change(RuntimeState::Starting);
            if let Err(e) = supervisor.open() {
                self.change(RuntimeState::Failed(format!("{e:#}")));
                return Err(e);
            }
            self.change(supervisor.ready());
        }
        let thread = std::thread::Builder::new()
            .name("runtime".into())
            .spawn(move || supervisor.serve(commands_rx))
            .context("failed to spawn runtime thread")?;
        self.commands = Some(commands_tx);
        self.thread = Some(thread);
        Ok(())
    }

    /// Restarts the pipeline with `config` once the dictation in progress ends; a
    /// configuration that does not validate is rejected and the pipeline keeps its settings.
    pub fn reload(&self, config: Config) -> Result<()> {
        self.reloader()?.reload(config)
    }

    /// A handle another thread reloads the running runtime with.
    pub fn reloader(&self) -> Result<Reloader> {
        let commands = self.commands.clone().context("the runtime does not run")?;
        Ok(Reloader { commands })
    }

    /// Ends the pipeline at once, cancelling a recording in progress; the state is Stopped.
    pub fn stop(&mut self) {
        if let Some(commands) = self.commands.take() {
            let _ = commands.send(Command::Stop);
        }
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            log::error!("runtime: the runtime thread panicked");
        }
    }

    fn change(&self, state: RuntimeState) {
        lock(&self.observers).change(state);
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Reloads the runtime it came from, from any thread.
#[derive(Clone)]
pub struct Reloader {
    commands: Sender<Command>,
}

impl Reloader {
    /// As [`Runtime::reload`].
    pub fn reload(&self, config: Config) -> Result<()> {
        self.send_reload(config, None)
    }

    /// As [`Reloader::reload`], telling `listener` what becomes of this reload: whether
    /// it waits for the dictation in progress, and whether it was applied or rejected.
    pub fn reload_reporting(&self, config: Config, listener: ReloadListener) -> Result<()> {
        self.send_reload(config, Some(listener))
    }

    fn send_reload(&self, config: Config, mut listener: Option<ReloadListener>) -> Result<()> {
        if let Err(e) = config::validate(&config) {
            log_rejection(&e);
            tell(&mut listener, ReloadReport::Rejected(format!("{e:#}")));
            return Err(e);
        }
        self.commands
            .send(Command::Reload(Box::new(config), listener))
            .map_err(|_| anyhow::anyhow!("the runtime has stopped"))
    }
}

fn lock(observers: &Mutex<Observers>) -> MutexGuard<'_, Observers> {
    observers
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn tell(listener: &mut Option<ReloadListener>, report: ReloadReport) {
    if let Some(listener) = listener {
        listener(report);
    }
}

pub(crate) fn log_rejection(reason: &anyhow::Error) {
    log::warn!(
        "runtime: configuration rejected: {reason:#}; the running pipeline keeps its settings"
    );
}

/// A pipeline assembled and not yet started, with the channel it will read.
struct Next {
    pipeline: Pipeline,
    inputs: Sender<PipelineInput>,
    inbox: Receiver<PipelineInput>,
}

impl Next {
    fn assemble(assembler: &mut dyn PipelineAssembler, config: &Config) -> Result<Self> {
        let (inputs, inbox) = mpsc::channel();
        let pipeline = assembler.assemble(config, inputs.clone())?;
        Ok(Self { pipeline, inputs, inbox })
    }

    /// Runs the pipeline on a thread of its own; one that ends with an error or a panic is
    /// stopped in order and reported to `failures` under `generation`.
    fn launch(
        self,
        activity: &ActivityRegister,
        failures: Option<Sender<Command>>,
        generation: u64,
    ) -> Running {
        activity.publish(PipelineActivity::Idle);
        let mut pipeline = self.pipeline.with_activity_sink(Box::new(activity.clone()));
        let inbox = self.inbox;
        let thread = std::thread::Builder::new()
            .name("pipeline".into())
            .spawn(move || {
                let outcome =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pipeline.run(inbox)));
                let reason = match outcome {
                    Ok(Ok(())) => return,
                    Ok(Err(e)) => format!("{e:#}"),
                    Err(panic) => panic_message(panic.as_ref()),
                };
                log::error!("fatal: pipeline stopped: {reason}");
                pipeline.stop_after_failure();
                if let Some(failures) = failures {
                    let _ = failures.send(Command::PipelineFailed { generation, reason });
                }
            })
            .expect("failed to spawn pipeline thread");
        Running { inputs: self.inputs, thread }
    }
}

/// What a panic said, as far as it is text.
fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    match (panic.downcast_ref::<&str>(), panic.downcast_ref::<String>()) {
        (Some(message), _) => format!("panic: {message}"),
        (None, Some(message)) => format!("panic: {message}"),
        (None, None) => "panic".to_owned(),
    }
}

/// The pipeline that runs, and its input channel.
struct Running {
    inputs: Sender<PipelineInput>,
    thread: JoinHandle<()>,
}

impl Running {
    /// Ends the pipeline and waits until its parts are gone.
    fn shut_down(self) {
        self.end_with(PipelineInput::Shutdown);
    }

    /// Ends the pipeline because `permission` was revoked.
    fn revoke(self, permission: Permission) {
        self.end_with(PipelineInput::PermissionRevoked(permission));
    }

    /// Waits until a pipeline that ended on its own is gone.
    fn join(self) {
        if self.thread.join().is_err() {
            log::error!("runtime: the pipeline thread panicked");
        }
    }

    fn end_with(self, last_input: PipelineInput) {
        let _ = self.inputs.send(last_input);
        self.join();
    }
}

/// What a restart that waits for the dictation in progress learns meanwhile.
#[derive(PartialEq, Eq)]
enum Wait {
    Idle,
    Stopped,
}

/// The runtime thread: every reload and the stop run here, one after the other.
struct Supervisor {
    assembler: Box<dyn PipelineAssembler>,
    /// Spawned with the first pipeline, so it needs Input Monitoring only from then on.
    listener: Option<Box<dyn HotkeyListener>>,
    permissions: Option<Watched>,
    services: Option<Box<dyn LocalServices>>,
    models: Option<Box<dyn ModelsGate>>,
    /// The services were told what the configuration needs; not while models are missing.
    services_required: bool,
    /// A pipeline that failed to start is not started again by a permission check.
    failed: bool,
    observers: Arc<Mutex<Observers>>,
    router: SignalRouter,
    binding: HotkeyBinding,
    activity: ActivityRegister,
    pipeline: Option<Running>,
    config: Config,
    /// Where a pipeline reports that it failed; none in tests that build no runtime.
    failures: Option<Sender<Command>>,
    /// Numbers the pipelines, so a failure reported by a replaced one is ignored.
    generation: u64,
    /// When the pipeline was last started again after a failure.
    restarted_at: Option<Instant>,
}

impl Supervisor {
    #[expect(clippy::too_many_arguments, reason = "size baseline")]
    fn new(
        assembler: Box<dyn PipelineAssembler>,
        listener: Box<dyn HotkeyListener>,
        observers: Arc<Mutex<Observers>>,
        config: Config,
        permissions: Option<Watched>,
        services: Option<Box<dyn LocalServices>>,
        models: Option<Box<dyn ModelsGate>>,
    ) -> Self {
        let mut supervisor = Self {
            assembler,
            listener: Some(listener),
            permissions,
            services,
            models,
            services_required: false,
            failed: false,
            observers,
            router: SignalRouter::new(),
            binding: HotkeyBinding::new(config.hotkey.key),
            activity: ActivityRegister::default(),
            pipeline: None,
            config,
            failures: None,
            generation: 0,
            restarted_at: None,
        };
        supervisor.models_missing();
        supervisor
    }

    fn reporting_failures_to(mut self, failures: Sender<Command>) -> Self {
        self.failures = Some(failures);
        self
    }

    /// Runs `next` as the current pipeline, under a new generation.
    fn launch(&mut self, next: Next) -> Running {
        self.generation += 1;
        next.launch(&self.activity, self.failures.clone(), self.generation)
    }

    /// Starts a new pipeline after the current one failed; one that failed soon after such a
    /// restart fails the runtime instead.
    fn pipeline_failed(&mut self, generation: u64, reason: String) {
        if generation != self.generation {
            return;
        }
        self.router.detach();
        if let Some(running) = self.pipeline.take() {
            running.join();
        }
        let soon_again = self
            .restarted_at
            .is_some_and(|at| at.elapsed() < RESTART_WINDOW);
        if soon_again {
            self.failed = true;
            let failure = format!("the dictation pipeline stopped: {reason}");
            self.change(RuntimeState::Failed(failure));
            return;
        }
        log::warn!("runtime: starting a new pipeline after: {reason}");
        self.restarted_at = Some(Instant::now());
        self.start_pipeline();
    }

    /// Starts the pipeline; a start that fails leaves the runtime Failed.
    fn start_pipeline(&mut self) {
        self.change(RuntimeState::Starting);
        match self.open() {
            Ok(()) => self.change(self.ready()),
            Err(e) => {
                self.failed = true;
                self.change(RuntimeState::Failed(format!("{e:#}")));
            }
        }
    }

    /// Waiting for models when a file the configuration needs is missing; otherwise the
    /// services are told what it needs, once.
    fn models_missing(&mut self) -> Option<RuntimeState> {
        let missing = match &self.models {
            Some(models) => models.missing(&self.config),
            None => Vec::new(),
        };
        if let Some(models) = self.models.as_ref().filter(|_| !missing.is_empty()) {
            self.services_required = false;
            let folder = models.folder(&self.config);
            return Some(RuntimeState::WaitingForModels { folder, missing });
        }
        if !self.services_required {
            if let Some(services) = &mut self.services {
                services.require(&self.config);
            }
            self.services_required = true;
        }
        None
    }

    /// What keeps the pipeline from starting: a missing permission, then a missing model
    /// file, then a needed service that failed or does not answer yet; none when it may start.
    fn gate(&mut self) -> Option<RuntimeState> {
        let models = self.models_missing();
        if let Some(watched) = self.permissions.as_ref().filter(|w| !w.all_granted()) {
            return Some(watched.waiting());
        }
        if models.is_some() {
            return models;
        }
        let progress = self.services.as_ref()?.progress();
        if let Some(reason) = progress.failed {
            return Some(RuntimeState::Failed(reason));
        }
        (!progress.all_ready()).then_some(RuntimeState::StartingServices {
            ready: progress.ready,
            total: progress.total,
        })
    }

    /// Starts a pipeline from the configuration, and the hotkey listener with the first one.
    fn open(&mut self) -> Result<()> {
        let next = Next::assemble(self.assembler.as_mut(), &self.config)?;
        let running = self.launch(next);
        self.router.route_to(running.inputs.clone());
        self.pipeline = Some(running);
        if let Some(listener) = self.listener.take() {
            let (hotkey_tx, hotkey_rx) = mpsc::channel::<PttSignal>();
            listener
                .spawn(self.binding.clone(), hotkey_tx)
                .context("failed to spawn hotkey listener")?;
            self.router.spawn_forwarding(hotkey_rx);
        }
        Ok(())
    }

    /// The next command; none when the permissions or the services are due for a check first.
    fn next_command(&self, commands: &Receiver<Command>) -> Option<Command> {
        let pipeline_runs = self.pipeline.is_some();
        let permissions = self.permissions.as_ref().map(|w| w.interval(pipeline_runs));
        let services = (self.services.is_some() && !pipeline_runs).then_some(SERVICES_POLL);
        let models = (self.models.is_some() && !pipeline_runs).then_some(MODELS_POLL);
        let Some(interval) = permissions.into_iter().chain(services).chain(models).min() else {
            return Some(commands.recv().unwrap_or(Command::Stop));
        };
        match commands.recv_timeout(interval) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => Some(Command::Stop),
        }
    }

    /// Closes the pipeline when a permission is revoked, and starts it once every permission
    /// is granted and every needed service answers.
    fn check_gates(&mut self) {
        let pipeline_runs = self.pipeline.is_some();
        if let Some(watched) = &mut self.permissions {
            match watched.check(pipeline_runs) {
                Verdict::StillWaiting(state) => self.change_to_new(state),
                Verdict::Close(state) => {
                    self.close(&state);
                    self.change(state);
                }
                Verdict::Unchanged | Verdict::Open => {}
            }
        }
        if self.pipeline.is_some() || self.failed {
            return;
        }
        match self.gate() {
            Some(state) => self.change_to_new(state),
            None => self.start_pipeline(),
        }
    }

    /// Ends the pipeline for a revoked permission: a recording in progress is cancelled.
    fn close(&mut self, waiting: &RuntimeState) {
        self.router.detach();
        let Some(running) = self.pipeline.take() else {
            return;
        };
        match waiting {
            RuntimeState::WaitingForPermissions { revoked, .. } if !revoked.is_empty() => {
                running.revoke(revoked[0]);
            }
            _ => running.shut_down(),
        }
    }

    fn serve(mut self, commands: Receiver<Command>) {
        let mut pending = std::collections::VecDeque::new();
        loop {
            let next = pending.pop_front().or_else(|| self.next_command(&commands));
            let Some(command) = next else {
                self.check_gates();
                continue;
            };
            let stopped = match command {
                Command::Reload(config, mut listener) => {
                    let reload = self.reload(*config, &mut listener, &commands, &mut pending);
                    reload == Wait::Stopped
                }
                Command::Stop => true,
                Command::PipelineFailed { generation, reason } => {
                    self.pipeline_failed(generation, reason);
                    false
                }
            };
            if stopped {
                return self.stop();
            }
        }
    }

    fn reload(
        &mut self,
        config: Config,
        listener: &mut Option<ReloadListener>,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        let models_missing = self
            .models
            .as_ref()
            .is_some_and(|models| !models.missing(&config).is_empty());
        if !models_missing {
            if let Some(services) = &mut self.services {
                services.require(&config);
            }
            self.services_required = true;
        }
        if self.pipeline.is_none() {
            return self.keep_for_later(config);
        }
        if models_missing {
            return self.wait_for_models(config, listener, commands, pending);
        }
        self.change(RuntimeState::Restarting);
        if self.wait_for_services(commands, pending) == Wait::Stopped {
            return Wait::Stopped;
        }
        let next = match Next::assemble(self.assembler.as_mut(), &config) {
            Ok(next) => next,
            Err(e) => {
                log_rejection(&e);
                tell(listener, ReloadReport::Rejected(format!("{e:#}")));
                self.change(self.ready());
                return Wait::Idle;
            }
        };
        if self.wait_for_idle(listener, commands, pending) == Wait::Stopped {
            return Wait::Stopped;
        }
        self.end_pipeline(config);
        let running = self.launch(next);
        self.router.route_to(running.inputs.clone());
        self.pipeline = Some(running);
        self.change(self.ready());
        tell(listener, ReloadReport::Applied);
        Wait::Idle
    }

    /// Waits until every service the new configuration needs answers; with one failed, the
    /// running pipeline keeps its settings until a later reload.
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    fn wait_for_services(
        &self,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        let Some(services) = &self.services else {
            return Wait::Idle;
        };
        loop {
            let progress = services.progress();
            if progress.all_ready() || progress.failed.is_some() {
                if let Some(reason) = progress.failed {
                    log::warn!("runtime: a needed service failed: {reason}");
                }
                return Wait::Idle;
            }
            self.change_to_new(RuntimeState::StartingServices {
                ready: progress.ready,
                total: progress.total,
            });
            match commands.recv_timeout(SERVICES_POLL) {
                Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Wait::Stopped
                }
                Ok(reload) => pending.push_back(reload),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    /// Ends the pipeline once no dictation is in progress and waits for the models `config`
    /// needs; the services are told what it needs once they are present.
    fn wait_for_models(
        &mut self,
        config: Config,
        listener: &mut Option<ReloadListener>,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        self.change(RuntimeState::Restarting);
        if self.wait_for_idle(listener, commands, pending) == Wait::Stopped {
            return Wait::Stopped;
        }
        self.end_pipeline(config);
        if let Some(state) = self.models_missing() {
            self.change(state);
        }
        tell(listener, ReloadReport::Applied);
        Wait::Idle
    }

    /// Without a pipeline, the configuration waits for the next start.
    fn keep_for_later(&mut self, config: Config) -> Wait {
        self.end_pipeline(config);
        log::info!("runtime: configuration kept for when the pipeline starts");
        Wait::Idle
    }

    /// Ends the pipeline, if one runs, and keeps `config` as the configuration of the next
    /// one, its hotkey bound.
    fn end_pipeline(&mut self, config: Config) {
        if let Some(old) = self.pipeline.take() {
            old.shut_down();
        }
        self.binding.rebind(config.hotkey.key);
        self.config = config;
    }

    /// Waits until no dictation is in progress and detaches the router from the pipeline;
    /// a stop arriving meanwhile ends the wait, a reload waits its turn.
    fn wait_for_idle(
        &self,
        listener: &mut Option<ReloadListener>,
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        let mut told = false;
        loop {
            match commands.try_recv() {
                Ok(Command::Stop) | Err(mpsc::TryRecvError::Disconnected) => return Wait::Stopped,
                Ok(reload) => pending.push_back(reload),
                Err(mpsc::TryRecvError::Empty) => {}
            }
            let idle = self.activity.wait_idle(IDLE_POLL_INTERVAL) == PipelineActivity::Idle;
            if idle && self.detached_idle() {
                return Wait::Idle;
            }
            if !idle && !told {
                log::info!("runtime: restart waits for the dictation in progress");
                tell(listener, ReloadReport::WaitsForDictation);
                told = true;
            }
        }
    }

    /// Detaches the router and whether the pipeline is still idle after that: a signal
    /// forwarded just before the detach may still be on its way to the pipeline, so it gets
    /// the time to start its dictation, which the wait then keeps, the router attached again.
    fn detached_idle(&self) -> bool {
        self.router.detach();
        std::thread::sleep(IDLE_POLL_INTERVAL);
        if self.activity.current() == PipelineActivity::Idle {
            return true;
        }
        if let Some(running) = &self.pipeline {
            self.router.route_to(running.inputs.clone());
        }
        false
    }

    fn stop(mut self) {
        self.router.detach();
        if let Some(running) = self.pipeline.take() {
            running.shut_down();
        }
        if let Some(services) = &mut self.services {
            services.stop();
        }
        lock(&self.observers).change(RuntimeState::Stopped);
    }

    fn ready(&self) -> RuntimeState {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::hotkey_listener::MockHotkeyListener;
    use crate::core::application::pipeline::{PipelineConfig, PipelineParts};
    use crate::core::application::sound_player::MockSoundPlayer;
    use crate::core::application::text_paster::MockTextPaster;
    use crate::core::application::transcriber::MockTranscriber;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::domain::{HotKey, RecordingMode};
    use std::time::Instant;

    /// Longest the runtime thread may take to reach a state.
    const REACTION: Duration = Duration::from_secs(2);

    struct IdleAssembler;

    impl PipelineAssembler for IdleAssembler {
        fn assemble(&mut self, _cfg: &Config, _in: Sender<PipelineInput>) -> Result<Pipeline> {
            Ok(Pipeline::new(
                PipelineParts {
                    recorder: Box::new(MockAudioRecorder::new()),
                    transcriber: Box::new(MockTranscriber::new()),
                    paster: Box::new(MockTextPaster::new()),
                    sound_player: Box::new(MockSoundPlayer::new()),
                    visual_indicator: Box::new(MockVisualIndicator::new()),
                },
                PipelineConfig::default(),
            ))
        }
    }

    struct Recorded(Arc<Mutex<Vec<RuntimeState>>>);

    impl RuntimeObserver for Recorded {
        fn state_changed(&self, state: &RuntimeState) {
            self.0.lock().unwrap().push(state.clone());
        }
    }

    fn listener() -> Box<MockHotkeyListener> {
        let mut listener = MockHotkeyListener::new();
        listener
            .expect_spawn()
            .returning(|_, _| Ok(std::thread::spawn(|| {})));
        Box::new(listener)
    }

    fn with_hotkey(hotkey: HotKey) -> Config {
        let mut config = Config::default();
        config.hotkey.key = hotkey;
        config
    }

    fn ready(hotkey: HotKey) -> RuntimeState {
        RuntimeState::Ready { hotkey, recording_mode: RecordingMode::Toggle, voice_control: false }
    }

    #[test]
    fn should_tell_an_observer_every_state_of_a_start_a_reload_and_a_stop_in_order() {
        // Given
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener());
        runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
        // When
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        runtime.reload(with_hotkey(HotKey::F19)).unwrap();
        wait_for_state(&seen, &ready(HotKey::F19));
        runtime.stop();
        // Then
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                RuntimeState::Starting,
                ready(HotKey::F13),
                RuntimeState::Restarting,
                ready(HotKey::F19),
                RuntimeState::Stopped,
            ]
        );
    }

    struct FailingReassembly(usize);

    impl PipelineAssembler for FailingReassembly {
        fn assemble(&mut self, config: &Config, inputs: Sender<PipelineInput>) -> Result<Pipeline> {
            self.0 += 1;
            if self.0 > 1 {
                bail!("speech recognition service is not reachable");
            }
            IdleAssembler.assemble(config, inputs)
        }
    }

    fn reports_of(
        runtime: &Runtime,
        config: Config,
    ) -> (Result<()>, Arc<Mutex<Vec<ReloadReport>>>) {
        let reports = Arc::new(Mutex::new(Vec::new()));
        let told = Arc::clone(&reports);
        let result = runtime
            .reloader()
            .unwrap()
            .reload_reporting(config, Box::new(move |r| told.lock().unwrap().push(r)));
        (result, reports)
    }

    fn wait_for_report(reports: &Mutex<Vec<ReloadReport>>) -> Vec<ReloadReport> {
        wait_until(|| !reports.lock().unwrap().is_empty());
        reports.lock().unwrap().clone()
    }

    /// Whether `done` held within the reaction time.
    fn wait_until(done: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + REACTION;
        while !done() {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        true
    }

    #[test]
    fn should_report_a_reload_applied_once_the_new_pipeline_runs() {
        // Given
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener());
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        // When
        let (result, reports) = reports_of(&runtime, with_hotkey(HotKey::F19));
        // Then
        assert!(result.is_ok());
        assert_eq!(wait_for_report(&reports), vec![ReloadReport::Applied]);
        assert_eq!(runtime.state(), Some(ready(HotKey::F19)));
    }

    #[test]
    fn should_report_a_reload_rejected_with_the_reason_a_service_gave() {
        // Given
        let mut runtime = Runtime::new(Box::new(FailingReassembly(0)), listener());
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        // When
        let (_, reports) = reports_of(&runtime, with_hotkey(HotKey::F19));
        // Then
        let reports = wait_for_report(&reports);
        assert!(
            matches!(&reports[..], [ReloadReport::Rejected(reason)] if reason.contains("speech recognition")),
            "{reports:?}"
        );
    }

    #[test]
    fn should_report_a_configuration_that_does_not_validate_rejected_at_once() {
        // Given
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener());
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        let mut remote = with_hotkey(HotKey::F19);
        remote.transcription.base_url = "http://192.0.2.1:8081".into();
        // When
        let (result, reports) = reports_of(&runtime, remote);
        // Then
        assert!(result.is_err());
        assert!(matches!(
            &reports.lock().unwrap()[..],
            [ReloadReport::Rejected(_)]
        ));
    }

    #[test]
    fn should_tell_the_recording_mode_of_the_configuration_when_ready() {
        // Given
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener());
        let mut config = with_hotkey(HotKey::F13);
        config.hotkey.recording_mode = RecordingMode::Hold;
        // When
        runtime.start(config).unwrap();
        // Then
        assert!(matches!(
            runtime.state(),
            Some(RuntimeState::Ready { recording_mode: RecordingMode::Hold, .. })
        ));
    }

    #[test]
    fn should_tell_an_observer_registered_after_the_start_ready_at_once() {
        // Given
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener());
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        // When
        runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
        // Then
        assert_eq!(*seen.lock().unwrap(), vec![ready(HotKey::F13)]);
    }
    /// The model files still missing, shared with the test.
    #[derive(Clone, Default)]
    struct FakeModels(Arc<Mutex<Vec<&'static str>>>);

    impl FakeModels {
        fn missing_now(&self, files: &[&'static str]) {
            *self.0.lock().unwrap() = files.to_vec();
        }
    }

    impl ModelsGate for FakeModels {
        fn folder(&self, _config: &Config) -> std::path::PathBuf {
            "/models".into()
        }

        fn missing(&self, _config: &Config) -> Vec<&'static str> {
            self.0.lock().unwrap().clone()
        }
    }

    /// Services that answer as soon as they are required; how often they were.
    #[derive(Clone, Default)]
    /// Counts the requires; ready from the second look after the first require.
    struct FakeServices(Arc<Mutex<usize>>, Arc<Mutex<usize>>);

    impl LocalServices for FakeServices {
        fn require(&mut self, _config: &Config) {
            *self.0.lock().unwrap() += 1;
        }

        fn progress(&self) -> crate::core::domain::ServicesProgress {
            let required = *self.0.lock().unwrap() > 0;
            let mut looks = self.1.lock().unwrap();
            if required {
                *looks += 1;
            }
            crate::core::domain::ServicesProgress {
                ready: usize::from(*looks > 1),
                total: 1,
                failed: None,
            }
        }

        fn stop(&mut self) {}
    }

    fn waiting_for(files: &[&'static str]) -> RuntimeState {
        RuntimeState::WaitingForModels { folder: "/models".into(), missing: files.to_vec() }
    }

    fn wait_for_state(seen: &Mutex<Vec<RuntimeState>>, state: &RuntimeState) {
        let reached = wait_until(|| seen.lock().unwrap().last() == Some(state));
        assert!(reached, "seen: {:?}", seen.lock().unwrap());
    }

    #[test]
    fn should_wait_for_models_without_requiring_a_service() {
        // Given
        let seen = Arc::new(Mutex::new(Vec::new()));
        let models = FakeModels::default();
        models.missing_now(&["ggml-large-v3-turbo-q5_0.bin"]);
        let services = FakeServices::default();
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener())
            .with_services(Box::new(services.clone()))
            .with_models(Box::new(models));
        runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
        // When
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        // Then
        assert_eq!(
            *seen.lock().unwrap(),
            vec![waiting_for(&["ggml-large-v3-turbo-q5_0.bin"])]
        );
        assert_eq!(*services.0.lock().unwrap(), 0);
    }

    #[test]
    fn should_start_the_services_and_the_pipeline_once_the_models_are_present() {
        // Given
        let seen = Arc::new(Mutex::new(Vec::new()));
        let models = FakeModels::default();
        models.missing_now(&["ggml-large-v3-turbo-q5_0.bin"]);
        let services = FakeServices::default();
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener())
            .with_services(Box::new(services.clone()))
            .with_models(Box::new(models.clone()));
        runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        // When
        models.missing_now(&[]);
        wait_for_state(&seen, &ready(HotKey::F13));
        // Then
        assert_eq!(*services.0.lock().unwrap(), 1);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                waiting_for(&["ggml-large-v3-turbo-q5_0.bin"]),
                RuntimeState::StartingServices { ready: 0, total: 1 },
                RuntimeState::Starting,
                ready(HotKey::F13),
            ]
        );
    }

    #[test]
    fn should_end_the_pipeline_and_wait_when_a_reload_needs_missing_models() {
        // Given
        let seen = Arc::new(Mutex::new(Vec::new()));
        let models = FakeModels::default();
        let mut runtime = Runtime::new(Box::new(IdleAssembler), listener())
            .with_services(Box::new(FakeServices::default()))
            .with_models(Box::new(models.clone()));
        runtime.observe(Box::new(Recorded(Arc::clone(&seen))));
        runtime.start(with_hotkey(HotKey::F13)).unwrap();
        wait_for_state(&seen, &ready(HotKey::F13));
        seen.lock().unwrap().clear();
        // When
        models.missing_now(&["ggml-small.bin"]);
        runtime.reload(with_hotkey(HotKey::F19)).unwrap();
        wait_for_state(&seen, &waiting_for(&["ggml-small.bin"]));
        // Then
        assert_eq!(
            *seen.lock().unwrap(),
            vec![RuntimeState::Restarting, waiting_for(&["ggml-small.bin"]),]
        );
    }
}
