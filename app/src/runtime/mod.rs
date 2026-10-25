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

mod supervisor;
use supervisor::{Gates, Supervisor};

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
            Gates::new(
                self.permissions.take().map(Watched::start),
                self.services.take(),
                self.models.take(),
            ),
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
            .spawn(move || supervisor.serve(&commands_rx))
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
    ) -> Result<Running> {
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
            .context("failed to spawn the pipeline thread")?;
        Ok(Running { inputs: self.inputs, thread })
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

#[cfg(test)]
mod tests;
