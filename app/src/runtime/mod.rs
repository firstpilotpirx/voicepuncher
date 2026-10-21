//! The life of the application between launch and quit: the dictation pipeline starts from
//! the configuration, stops, and restarts with new settings while the process keeps running.

pub mod activity;
pub mod assembler;
pub mod observer;
pub mod overlay;
pub mod permissions;
pub mod report;
pub mod router;
pub mod signals;
pub mod wiring;

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::config::{self, Config};
use crate::core::application::pipeline::Pipeline;
use crate::core::application::{
    HotkeyListener, PipelineActivity, PipelineActivitySink, PipelineInput,
};
use crate::core::domain::{HotkeyBinding, Permission, PttSignal};

use permissions::{Verdict, Watched};

pub use activity::ActivityRegister;
pub use assembler::PipelineAssembler;
pub use observer::{LogObserver, Observers, RuntimeObserver, RuntimeState};
pub use overlay::SharedVisualIndicator;
pub use permissions::PermissionsWatch;
pub use report::{ReloadListener, ReloadReport};
pub use router::SignalRouter;

/// How often a restart checks whether the dictation in progress has ended.
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Owns the hotkey listener for the life of the process and one pipeline at a time.
pub struct Runtime {
    assembler: Option<Box<dyn PipelineAssembler>>,
    listener: Option<Box<dyn HotkeyListener>>,
    permissions: Option<PermissionsWatch>,
    observers: Arc<Mutex<Observers>>,
    commands: Option<Sender<Command>>,
    thread: Option<JoinHandle<()>>,
}

enum Command {
    Reload(Box<Config>, Option<ReloadListener>),
    Stop,
}

impl Runtime {
    pub fn new(assembler: Box<dyn PipelineAssembler>, listener: Box<dyn HotkeyListener>) -> Self {
        Self {
            assembler: Some(assembler),
            listener: Some(listener),
            permissions: None,
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

    /// Tells `observer` of every state change from now on, and of the current state at once.
    pub fn observe(&self, observer: Box<dyn RuntimeObserver>) {
        lock(&self.observers).register(observer);
    }

    /// The current state; none before the start.
    pub fn state(&self) -> Option<RuntimeState> {
        lock(&self.observers).current().cloned()
    }

    /// Starts the pipeline from `config` and returns once it runs, or with the reason it
    /// cannot; the state is then Ready or Failed. With a permission missing it returns at
    /// once in the state Waiting for permissions, and starts the pipeline once all are granted.
    pub fn start(&mut self, config: Config) -> Result<()> {
        let (Some(assembler), Some(listener)) = (self.assembler.take(), self.listener.take())
        else {
            bail!("the runtime was started already");
        };
        let mut supervisor = Supervisor::new(
            assembler,
            listener,
            Arc::clone(&self.observers),
            config,
            self.permissions.take().map(Watched::start),
        );
        if supervisor.waits_for_permissions() {
            self.change(supervisor.waiting());
        } else {
            self.change(RuntimeState::Starting);
            if let Err(e) = supervisor.open() {
                self.change(RuntimeState::Failed(format!("{e:#}")));
                return Err(e);
            }
            self.change(supervisor.ready());
        }
        let (commands_tx, commands_rx) = mpsc::channel();
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
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                log::error!("runtime: the runtime thread panicked");
            }
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
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        Ok(Self {
            pipeline,
            inputs,
            inbox,
        })
    }

    fn launch(self, activity: &ActivityRegister) -> Running {
        activity.publish(PipelineActivity::Idle);
        let mut pipeline = self.pipeline.with_activity_sink(Box::new(activity.clone()));
        let inbox = self.inbox;
        let thread = std::thread::Builder::new()
            .name("pipeline".into())
            .spawn(move || {
                pipeline.run(inbox).unwrap_or_else(|e| {
                    log::error!("fatal: {e:#}");
                    std::process::exit(1);
                });
            })
            .expect("failed to spawn pipeline thread");
        Running {
            inputs: self.inputs,
            thread,
        }
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

    fn end_with(self, last_input: PipelineInput) {
        let _ = self.inputs.send(last_input);
        if self.thread.join().is_err() {
            log::error!("runtime: the pipeline thread panicked");
        }
    }
}

/// What a restart that waits for the dictation in progress learns meanwhile.
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
    /// A pipeline that failed to start is not started again by a permission check.
    failed: bool,
    observers: Arc<Mutex<Observers>>,
    router: SignalRouter,
    binding: HotkeyBinding,
    activity: ActivityRegister,
    pipeline: Option<Running>,
    config: Config,
}

impl Supervisor {
    fn new(
        assembler: Box<dyn PipelineAssembler>,
        listener: Box<dyn HotkeyListener>,
        observers: Arc<Mutex<Observers>>,
        config: Config,
        permissions: Option<Watched>,
    ) -> Self {
        Self {
            assembler,
            listener: Some(listener),
            permissions,
            failed: false,
            observers,
            router: SignalRouter::new(),
            binding: HotkeyBinding::new(config.hotkey.key),
            activity: ActivityRegister::default(),
            pipeline: None,
            config,
        }
    }

    fn waits_for_permissions(&self) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|watched| !watched.all_granted())
    }

    fn waiting(&self) -> RuntimeState {
        self.permissions
            .as_ref()
            .map_or(RuntimeState::Starting, Watched::waiting)
    }

    /// Starts a pipeline from the configuration, and the hotkey listener with the first one.
    fn open(&mut self) -> Result<()> {
        let running = Next::assemble(self.assembler.as_mut(), &self.config)?.launch(&self.activity);
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

    /// The next command; none when the permissions are due for a check first.
    fn next_command(&self, commands: &Receiver<Command>) -> Option<Command> {
        let Some(watched) = &self.permissions else {
            return Some(commands.recv().unwrap_or(Command::Stop));
        };
        match commands.recv_timeout(watched.interval(self.pipeline.is_some())) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => Some(Command::Stop),
        }
    }

    fn check_permissions(&mut self) {
        let pipeline_runs = self.pipeline.is_some();
        let Some(watched) = &mut self.permissions else {
            return;
        };
        match watched.check(pipeline_runs) {
            Verdict::Unchanged => {}
            Verdict::StillWaiting(state) => self.change(state),
            Verdict::Close(state) => {
                self.close(&state);
                self.change(state);
            }
            Verdict::Open if !self.failed => {
                self.change(RuntimeState::Starting);
                match self.open() {
                    Ok(()) => self.change(self.ready()),
                    Err(e) => {
                        self.failed = true;
                        self.change(RuntimeState::Failed(format!("{e:#}")));
                    }
                }
            }
            Verdict::Open => {}
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
            let command = match pending.pop_front() {
                Some(command) => command,
                None => match self.next_command(&commands) {
                    Some(command) => command,
                    None => {
                        self.check_permissions();
                        continue;
                    }
                },
            };
            match command {
                Command::Reload(config, mut listener) => {
                    let reload = self.reload(*config, &mut listener, &commands, &mut pending);
                    if let Wait::Stopped = reload {
                        return self.stop();
                    }
                }
                Command::Stop => return self.stop(),
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
        if self.pipeline.is_none() {
            return self.keep_for_later(config);
        }
        self.change(RuntimeState::Restarting);
        let next = match Next::assemble(self.assembler.as_mut(), &config) {
            Ok(next) => next,
            Err(e) => {
                log_rejection(&e);
                tell(listener, ReloadReport::Rejected(format!("{e:#}")));
                self.change(self.ready());
                return Wait::Idle;
            }
        };
        if let Wait::Stopped = self.wait_for_idle(listener, commands, pending) {
            return Wait::Stopped;
        }
        if let Some(old) = self.pipeline.take() {
            old.shut_down();
        }
        self.binding.rebind(config.hotkey.key);
        self.config = config;
        let running = next.launch(&self.activity);
        self.router.route_to(running.inputs.clone());
        self.pipeline = Some(running);
        self.change(self.ready());
        tell(listener, ReloadReport::Applied);
        Wait::Idle
    }

    /// Without a pipeline, the configuration waits for the next start.
    fn keep_for_later(&mut self, config: Config) -> Wait {
        self.binding.rebind(config.hotkey.key);
        self.config = config;
        log::info!("runtime: configuration kept for when the pipeline starts");
        Wait::Idle
    }

    /// Waits until no dictation is in progress and detaches the router from the pipeline;
    /// a stop arriving meanwhile ends the wait, a reload waits its turn.
    fn wait_for_idle(
        &mut self,
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
            if self.activity.current() == PipelineActivity::Idle {
                self.router.detach();
                // a signal forwarded just before the detach may still be on its way to the
                // pipeline: give it the time to start its dictation, which the wait then keeps
                std::thread::sleep(IDLE_POLL_INTERVAL);
                if self.activity.current() == PipelineActivity::Idle {
                    return Wait::Idle;
                }
                if let Some(running) = &self.pipeline {
                    self.router.route_to(running.inputs.clone());
                }
            }
            if !told {
                log::info!("runtime: restart waits for the dictation in progress");
                tell(listener, ReloadReport::WaitsForDictation);
                told = true;
            }
            std::thread::sleep(IDLE_POLL_INTERVAL);
        }
    }

    fn stop(mut self) {
        self.router.detach();
        if let Some(running) = self.pipeline.take() {
            running.shut_down();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::audio_recorder::MockAudioRecorder;
    use crate::core::application::hotkey_listener::MockHotkeyListener;
    use crate::core::application::pipeline::PipelineConfig;
    use crate::core::application::sound_player::MockSoundPlayer;
    use crate::core::application::text_paster::MockTextPaster;
    use crate::core::application::text_refiner::MockTextRefiner;
    use crate::core::application::transcriber::MockTranscriber;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::application::ChunkingCfg;
    use crate::core::domain::{HotKey, RecordingMode};
    use std::time::Instant;

    /// Longest the runtime thread may take to reach a state.
    const REACTION: Duration = Duration::from_secs(2);

    struct IdleAssembler;

    impl PipelineAssembler for IdleAssembler {
        fn assemble(
            &mut self,
            _config: &Config,
            _inputs: Sender<PipelineInput>,
        ) -> Result<Pipeline> {
            Ok(Pipeline::new(
                Box::new(MockAudioRecorder::new()),
                Box::new(MockTranscriber::new()),
                Box::new(MockTextRefiner::new()),
                Box::new(MockTextPaster::new()),
                Box::new(MockSoundPlayer::new()),
                Box::new(MockVisualIndicator::new()),
                PipelineConfig {
                    min_press_ms: 0,
                    recording_mode: RecordingMode::Toggle,
                    chunking: ChunkingCfg::default(),
                },
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
        RuntimeState::Ready {
            hotkey,
            recording_mode: RecordingMode::Toggle,
            voice_control: false,
        }
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
        let deadline = Instant::now() + REACTION;
        while seen.lock().unwrap().last() != Some(&ready(HotKey::F19)) {
            assert!(
                Instant::now() < deadline,
                "seen: {:?}",
                seen.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
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
        let deadline = Instant::now() + REACTION;
        while reports.lock().unwrap().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        reports.lock().unwrap().clone()
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
            Some(RuntimeState::Ready {
                recording_mode: RecordingMode::Hold,
                ..
            })
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
}
