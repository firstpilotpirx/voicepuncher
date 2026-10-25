//! The runtime run: the real runtime with stand-in parts and its event log.

use super::{
    bail, failures, models, services, Arc, Config, Context, Duration, Event, Events, FakeKeyboard,
    Generation, HotKey, Instant, IntegrationWorld, Mutex, Permission, Permissions,
    PermissionsWatch, PipelineAssembler, RecordingMode, RecordingObserver, Reloader, Result,
    Runtime, RuntimeState, StandInAssembler, FIRST_HOTKEY, PERMISSIONS_CHECK, REACTION, SETTLE,
};

/// What an app-runtime scenario shares with the runtime and its pipelines.
pub struct RuntimeRun {
    pub(crate) events: Events,
    pub(crate) config: Config,
    pub(crate) services_answer: Arc<Mutex<bool>>,
    pub(crate) text_field_focused: Arc<Mutex<bool>>,
    pub(crate) keyboard: FakeKeyboard,
    pub(crate) runtime: Option<Runtime>,
    pub(crate) started: Option<Result<()>>,
    pub(crate) observed: Observed,
    pub(crate) permissions: Permissions,
    pub(crate) services: Option<services::FakeServices>,
    pub(crate) models: Option<models::ModelsFolder>,
    pub(crate) panics: failures::PanicPlan,
}

/// What the scenario saw of the process: which one it is and the warnings of a rejected
/// configuration.
pub(crate) struct Observed {
    pub(crate) process: u32,
    pub(crate) rejection_log: Vec<String>,
}

impl std::fmt::Debug for RuntimeRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeRun")
            .field("events", &self.events())
            .finish()
    }
}

impl Default for RuntimeRun {
    fn default() -> Self {
        // the logger is the process's one; the voice-control steps install it
        crate::steps::voice_control::pipeline_log();
        Self {
            events: Events::default(),
            config: config_with(FIRST_HOTKEY),
            services_answer: Arc::new(Mutex::new(true)),
            text_field_focused: Arc::new(Mutex::new(true)),
            keyboard: FakeKeyboard::default(),
            runtime: None,
            started: None,
            observed: Observed { process: std::process::id(), rejection_log: Vec::new() },
            permissions: Permissions::default(),
            services: None,
            models: None,
            panics: failures::PanicPlan::default(),
        }
    }
}

impl Drop for RuntimeRun {
    fn drop(&mut self) {
        if let Some(mut runtime) = self.runtime.take() {
            runtime.stop();
        }
    }
}

pub(super) fn config_with(hotkey: HotKey) -> Config {
    let mut config = Config::default();
    config.hotkey.key = hotkey;
    config.hotkey.min_press_ms = 0;
    config.hotkey.recording_mode = RecordingMode::Toggle;
    config
}

impl RuntimeRun {
    /// Counts the dictations pasted so far whenever it is called, from any thread.
    pub(crate) fn paste_counter(&self) -> impl Fn() -> usize + Send + 'static {
        let events = Arc::clone(&self.events);
        move || {
            let events = events.lock().unwrap();
            events
                .iter()
                .filter(|e| matches!(e, Event::Pasted(_)))
                .count()
        }
    }

    pub(crate) fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub(crate) fn start(&mut self) {
        let assembler = self.assembler();
        let mut runtime = Runtime::new(Box::new(assembler), Box::new(self.keyboard.clone()))
            .with_permissions(PermissionsWatch {
                gate: self.permissions.gate(),
                while_ready: PERMISSIONS_CHECK,
                while_waiting: PERMISSIONS_CHECK,
            });
        if let Some(services) = &self.services {
            runtime = runtime.with_services(Box::new(services.clone()));
        }
        if let Some(models) = &self.models {
            runtime = runtime.with_models(models.gate());
        }
        runtime.observe(Box::new(RecordingObserver(Arc::clone(&self.events))));
        self.started = Some(runtime.start(self.config.clone()));
        self.runtime = Some(runtime);
    }

    pub(crate) fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    pub(crate) fn launch(&mut self) {
        self.start();
    }

    pub(crate) fn start_waiting(&mut self) -> Result<()> {
        self.start();
        self.state_becomes(|s| matches!(s, RuntimeState::WaitingForPermissions { .. }))
    }

    /// The start returned, and the process still runs it.
    pub(crate) fn started_ok(&mut self) -> Result<()> {
        self.started.take().context("the runtime was not started")?
    }

    pub(crate) fn assembled(&self, generation: Generation) -> Result<()> {
        self.wait_for(|e| matches!(e, Event::Assembled(g, _) if *g == generation))?;
        self.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))
    }

    pub(crate) fn same_process(&self) -> bool {
        self.observed.process == std::process::id()
    }

    pub(crate) fn ever_waited_for_permissions(&self) -> bool {
        self.count(|e| matches!(e, Event::State(RuntimeState::WaitingForPermissions { .. }))) > 0
    }

    pub(crate) fn waits_for(&self, missing: &[Permission], revoked: &[Permission]) -> Result<()> {
        self.state_becomes(|s| {
            *s == RuntimeState::WaitingForPermissions {
                missing: missing.to_vec(),
                revoked: revoked.to_vec(),
            }
        })
    }

    pub(crate) fn pipeline_dropped(&self, generation: Generation) -> Result<()> {
        self.wait_for(|e| *e == Event::PipelineDropped(generation))?;
        Ok(())
    }

    pub(crate) fn cancelled_and_nothing_pasted(&self) -> Result<()> {
        self.wait_for(|e| *e == Event::CancelPlayed(1))?;
        std::thread::sleep(SETTLE);
        if self.count(|e| matches!(e, Event::Pasted(_))) > 0 {
            bail!("text was pasted: {:?}", self.events());
        }
        Ok(())
    }

    pub(crate) fn notice(&self, generation: Generation) -> Result<String> {
        match self.wait_for(|e| matches!(e, Event::Notice(g, _) if *g == generation))? {
            Event::Notice(_, message) => Ok(message),
            _ => unreachable!("filtered above"),
        }
    }

    pub(crate) fn no_pipeline_runs(&self) -> Result<()> {
        if self.recordings_after(FIRST_HOTKEY) != 0 || self.latest_generation().is_some() {
            bail!("a pipeline runs: {:?}", self.events());
        }
        Ok(())
    }

    /// Transcribes a recording through a pipeline of the same stand-ins, as file mode does.
    pub(crate) fn transcribe_file(&self) -> Result<()> {
        let (inputs, _) = std::sync::mpsc::channel();
        let mut pipeline = self.assembler().assemble(&self.config, inputs)?;
        pipeline.run_once(voicepuncher::test_doubles::one_second_of_sound())?;
        Ok(())
    }

    pub(crate) fn printed(&self) -> Result<()> {
        self.wait_for(|e| matches!(e, Event::Pasted(_)))?;
        Ok(())
    }

    fn assembler(&self) -> StandInAssembler {
        StandInAssembler {
            events: Arc::clone(&self.events),
            services_answer: Arc::clone(&self.services_answer),
            text_field_focused: Arc::clone(&self.text_field_focused),
            panics: Arc::clone(&self.panics),
            generation: 0,
        }
    }

    pub(crate) fn start_ready(&mut self) -> Result<()> {
        self.start();
        self.wait_for(|e| matches!(e, Event::State(RuntimeState::Ready { .. })))?;
        Ok(())
    }

    pub(crate) fn runtime(&self) -> Result<&Runtime> {
        self.runtime.as_ref().context("the runtime was not started")
    }

    pub(crate) fn reload(&self, config: Config) -> Result<()> {
        self.runtime()?.reload(config)
    }

    pub(crate) fn wait_for(&self, wanted: impl Fn(&Event) -> bool) -> Result<Event> {
        self.wait_for_nth(1, wanted)
    }

    pub(crate) fn wait_for_nth(
        &self,
        nth: usize,
        wanted: impl Fn(&Event) -> bool,
    ) -> Result<Event> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(event) = self.events().into_iter().filter(&wanted).nth(nth - 1) {
                return Ok(event);
            }
            if Instant::now() >= deadline {
                bail!("not seen within {REACTION:?}; events: {:?}", self.events());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub(crate) fn position(&self, wanted: impl Fn(&Event) -> bool) -> Result<usize> {
        self.events()
            .iter()
            .position(wanted)
            .with_context(|| format!("not in the log: {:?}", self.events()))
    }

    pub(crate) fn count(&self, wanted: impl Fn(&Event) -> bool) -> usize {
        self.events().iter().filter(|e| wanted(e)).count()
    }

    pub(crate) fn last_state(&self) -> Option<RuntimeState> {
        self.events().into_iter().rev().find_map(|e| match e {
            Event::State(state) => Some(state),
            _ => None,
        })
    }

    pub(crate) fn state_becomes(&self, wanted: impl Fn(&RuntimeState) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        loop {
            if self.last_state().as_ref().is_some_and(&wanted) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "runtime state is {:?}; events: {:?}",
                    self.last_state(),
                    self.events()
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Starts the runtime from `config` and waits until it is Ready.
    pub(crate) fn start_from(&mut self, config: Config) -> Result<()> {
        self.config = config;
        self.start_ready()
    }

    pub(crate) fn reloader(&self) -> Result<Reloader> {
        self.runtime()?.reloader()
    }

    /// Whether a press of `key` starts a recording on the running pipeline.
    pub(crate) fn hotkey_records(&self, key: HotKey) -> bool {
        self.recordings_after(key) == 1
    }

    /// Ends the recording `key` started and waits until it was pasted.
    pub(crate) fn finish_dictation(&self, key: HotKey) -> Result<()> {
        let before = self.count(|e| matches!(e, Event::Pasted(_)));
        self.keyboard.press(key);
        self.wait_for_nth(before + 1, |e| matches!(e, Event::Pasted(_)))?;
        Ok(())
    }

    /// The start sound of the newest pipeline.
    pub(crate) fn newest_start_sound(&self) -> Option<String> {
        self.events().into_iter().rev().find_map(|e| match e {
            Event::Configured(_, sound) => Some(sound),
            _ => None,
        })
    }

    /// From now on every speech recognition service the runtime probes is silent.
    pub(crate) fn services_stop_answering(&self) {
        *self.services_answer.lock().unwrap() = false;
    }

    pub(crate) fn latest_generation(&self) -> Option<Generation> {
        self.events().into_iter().rev().find_map(|e| match e {
            Event::Assembled(generation, _) => Some(generation),
            _ => None,
        })
    }

    /// Starts a recording on the running pipeline and waits until it records.
    pub(crate) fn start_recording(&self) -> Result<Generation> {
        let before = self.count(|e| matches!(e, Event::RecordingStarted(_)));
        self.keyboard.press(self.config.hotkey.key);
        self.wait_for_nth(before + 1, |e| matches!(e, Event::RecordingStarted(_)))
            .map(|e| match e {
                Event::RecordingStarted(generation) => generation,
                _ => unreachable!("filtered above"),
            })
    }

    pub(crate) fn recordings_after(&self, key: HotKey) -> usize {
        let before = self.count(|e| matches!(e, Event::RecordingStarted(_)));
        self.keyboard.press(key);
        std::thread::sleep(SETTLE);
        self.count(|e| matches!(e, Event::RecordingStarted(_))) - before
    }
}

pub(crate) fn run(world: &mut IntegrationWorld) -> &mut RuntimeRun {
    world.app.runtime.get_or_insert_with(RuntimeRun::default)
}
