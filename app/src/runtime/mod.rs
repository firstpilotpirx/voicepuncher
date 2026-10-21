//! The life of the application between launch and quit: the dictation pipeline starts from
//! the configuration, stops, and restarts with new settings while the process keeps running.

pub mod activity;
pub mod assembler;
pub mod observer;
pub mod overlay;
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
use crate::core::domain::{HotkeyBinding, PttSignal};

pub use activity::ActivityRegister;
pub use assembler::PipelineAssembler;
pub use observer::{LogObserver, Observers, RuntimeObserver, RuntimeState};
pub use overlay::SharedVisualIndicator;
pub use router::SignalRouter;

/// How often a restart checks whether the dictation in progress has ended.
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Owns the hotkey listener for the life of the process and one pipeline at a time.
pub struct Runtime {
    assembler: Option<Box<dyn PipelineAssembler>>,
    listener: Option<Box<dyn HotkeyListener>>,
    observers: Arc<Mutex<Observers>>,
    commands: Option<Sender<Command>>,
    thread: Option<JoinHandle<()>>,
}

enum Command {
    Reload(Box<Config>),
    Stop,
}

impl Runtime {
    pub fn new(assembler: Box<dyn PipelineAssembler>, listener: Box<dyn HotkeyListener>) -> Self {
        Self {
            assembler: Some(assembler),
            listener: Some(listener),
            observers: Arc::default(),
            commands: None,
            thread: None,
        }
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
    /// cannot; the state is then Ready or Failed.
    pub fn start(&mut self, config: Config) -> Result<()> {
        let (Some(mut assembler), Some(listener)) = (self.assembler.take(), self.listener.take())
        else {
            bail!("the runtime was started already");
        };
        self.change(RuntimeState::Starting);
        let started = (|| {
            let activity = ActivityRegister::default();
            let pipeline = Next::assemble(assembler.as_mut(), &config)?.launch(&activity);
            let router = SignalRouter::new();
            router.route_to(pipeline.inputs.clone());
            let binding = HotkeyBinding::new(config.hotkey.key);
            let (hotkey_tx, hotkey_rx) = mpsc::channel::<PttSignal>();
            listener
                .spawn(binding.clone(), hotkey_tx)
                .context("failed to spawn hotkey listener")?;
            router.spawn_forwarding(hotkey_rx);
            Ok(Supervisor {
                assembler,
                observers: Arc::clone(&self.observers),
                router,
                binding,
                activity,
                pipeline: Some(pipeline),
                config,
            })
        })();
        let supervisor = match started {
            Ok(supervisor) => supervisor,
            Err(e) => {
                self.change(RuntimeState::Failed(format!("{e:#}")));
                return Err(e);
            }
        };
        self.change(supervisor.ready());
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
        if let Err(e) = config::validate(&config) {
            log_rejection(&e);
            return Err(e);
        }
        self.commands
            .send(Command::Reload(Box::new(config)))
            .map_err(|_| anyhow::anyhow!("the runtime has stopped"))
    }
}

fn lock(observers: &Mutex<Observers>) -> MutexGuard<'_, Observers> {
    observers
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        let _ = self.inputs.send(PipelineInput::Shutdown);
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
    observers: Arc<Mutex<Observers>>,
    router: SignalRouter,
    binding: HotkeyBinding,
    activity: ActivityRegister,
    pipeline: Option<Running>,
    config: Config,
}

impl Supervisor {
    fn serve(mut self, commands: Receiver<Command>) {
        let mut pending = std::collections::VecDeque::new();
        loop {
            let command = match pending.pop_front() {
                Some(command) => command,
                None => commands.recv().unwrap_or(Command::Stop),
            };
            match command {
                Command::Reload(config) => {
                    if let Wait::Stopped = self.reload(*config, &commands, &mut pending) {
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
        commands: &Receiver<Command>,
        pending: &mut std::collections::VecDeque<Command>,
    ) -> Wait {
        self.change(RuntimeState::Restarting);
        let next = match Next::assemble(self.assembler.as_mut(), &config) {
            Ok(next) => next,
            Err(e) => {
                log_rejection(&e);
                self.change(self.ready());
                return Wait::Idle;
            }
        };
        if let Wait::Stopped = self.wait_for_idle(commands, pending) {
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
        Wait::Idle
    }

    /// Waits until no dictation is in progress and detaches the router from the pipeline;
    /// a stop arriving meanwhile ends the wait, a reload waits its turn.
    fn wait_for_idle(
        &mut self,
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
