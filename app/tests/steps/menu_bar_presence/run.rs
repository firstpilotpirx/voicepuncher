//! The menu bar run: the runtime and the status item with their doubles.

use super::{
    bail, mpsc, services, Arc, Config, Context, FakeKeyboard, Flag, HeldStart, HotKey, IconState,
    Icons, Instant, IntegrationWorld, MacosMenuActions, MenuModel, Mutex, PipelineInput, PttSignal,
    QuietOverlay, QuitEvent, Receiver, RecordingMode, Result, Runtime, RuntimeObserver,
    RuntimeState, Sender, SharedVisualIndicator, StandInAssembler, StatusItemCell,
    StatusItemSurface, FOCUS_WAIT_SECONDS, NOTICE_LIFETIME, POLL, REACTION,
};

/// Records every icon the status item draws, with the moment it was drawn.
pub(super) struct SurfaceDouble(Icons);

impl StatusItemSurface for SurfaceDouble {
    fn show_icon(&self, icon: IconState) {
        self.0.lock().unwrap().push((Instant::now(), icon));
    }
}

/// What a menu-bar-presence scenario shares with the runtime, its pipelines and the status item.
pub struct MenuBarRun {
    pub(crate) drawn: Drawn,
    pub(crate) cell: StatusItemCell,
    pub(crate) config: Config,
    pub(crate) parts: Parts,
    keyboard: FakeKeyboard,
    pub(crate) runtime: Option<Runtime>,
    pub(crate) starting: Option<HeldStart>,
    pub(crate) started: Option<Result<()>>,
    pub(crate) menu: Option<MenuModel>,
    pub(crate) chosen: Chosen,
    /// The menu's actions once the user chose one; they own the runtime.
    pub(crate) actions: Option<MacosMenuActions>,
    pub(crate) services: ServicesShown,
}

/// The icons the status item drew, each when it was drawn.
pub(crate) struct Drawn {
    icons: Icons,
    /// How many icons were drawn before the scenario's action.
    pub(crate) before: usize,
}

/// What the user's choices in the menu did.
#[derive(Default)]
pub(crate) struct Chosen {
    /// What happened when the user chose Quit, in order.
    pub(crate) quit_log: Arc<Mutex<Vec<QuitEvent>>>,
    pub(crate) settings_opens: Arc<Mutex<usize>>,
    pub(crate) history_opens: Arc<Mutex<usize>>,
}

/// The services the menu shows.
#[derive(Default)]
pub(crate) struct ServicesShown {
    /// The services the menu's actions show.
    pub(crate) view: Option<Arc<services::ViewDouble>>,
    /// The Services submenu as the user opened it.
    pub(crate) menu: Option<services::ServiceMenu>,
}

/// What the stand-ins read and the scenario sets.
#[derive(Clone, Default)]
pub(crate) struct Parts {
    pub(crate) transcription_fails: Flag,
    pub(crate) transcription_held: Flag,
    pub(crate) text_field_focused: Flag,
    pub(crate) services_answer: Flag,
    pub(crate) cancel_played: Flag,
    pub(crate) pasted: Flag,
    pub(crate) inputs: Arc<Mutex<Option<Sender<PipelineInput>>>>,
}

impl std::fmt::Debug for MenuBarRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuBarRun")
            .field("icons", &self.icon_history())
            .finish()
    }
}

impl Default for MenuBarRun {
    fn default() -> Self {
        let icons = Icons::default();
        let cell =
            StatusItemCell::new(Box::new(SurfaceDouble(Arc::clone(&icons))), NOTICE_LIFETIME);
        let parts = Parts::default();
        *parts.text_field_focused.lock().unwrap() = true;
        *parts.services_answer.lock().unwrap() = true;
        Self {
            drawn: Drawn { icons, before: 0 },
            cell,
            config: config_with(HotKey::RightOption, RecordingMode::Toggle),
            parts,
            keyboard: FakeKeyboard::default(),
            runtime: None,
            starting: None,
            started: None,
            menu: None,
            chosen: Chosen::default(),
            actions: None,
            services: ServicesShown::default(),
        }
    }
}

impl Drop for MenuBarRun {
    fn drop(&mut self) {
        *self.parts.transcription_held.lock().unwrap() = false;
        if let Some((gate, starting)) = self.starting.take() {
            let _ = gate.send(true);
            if let Ok((runtime, _)) = starting.join() {
                self.runtime = Some(runtime);
            }
        }
        if let Some(mut runtime) = self.runtime.take() {
            runtime.stop();
        }
    }
}

pub(super) fn config_with(hotkey: HotKey, mode: RecordingMode) -> Config {
    let mut config = Config::default();
    config.hotkey.key = hotkey;
    config.hotkey.min_press_ms = 0;
    config.hotkey.recording_mode = mode;
    config.paste.focus_wait_seconds = FOCUS_WAIT_SECONDS;
    config
}

impl MenuBarRun {
    pub(crate) fn runtime_parts(&self, gate: Option<Receiver<bool>>) -> Runtime {
        let overlay = SharedVisualIndicator::new(Box::new(self.cell.indicator(QuietOverlay)));
        let assembler = StandInAssembler { parts: self.parts.clone(), overlay, gate };
        let runtime = Runtime::new(Box::new(assembler), Box::new(self.keyboard.clone()));
        runtime.observe(Box::new(self.cell.observer()));
        runtime.observe(Box::new(QuitObserver(Arc::clone(&self.chosen.quit_log))));
        runtime
    }

    pub(crate) fn start(&mut self) {
        let mut runtime = self.runtime_parts(None);
        self.started = Some(runtime.start(self.config.clone()));
        self.runtime = Some(runtime);
    }

    pub(crate) fn start_ready(&mut self) -> Result<()> {
        self.start();
        self.started_ok()?;
        self.icon_becomes(|icon| matches!(icon, IconState::Ready | IconState::Listening))
    }

    pub(crate) fn started_ok(&self) -> Result<()> {
        match &self.started {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => bail!("the runtime did not start: {e:#}"),
            None => bail!("the runtime was not started"),
        }
    }

    /// Starts the runtime on a thread of its own, held while it assembles its pipeline.
    pub(crate) fn start_held(&mut self) {
        let (gate_tx, gate_rx) = mpsc::channel();
        let mut runtime = self.runtime_parts(Some(gate_rx));
        let config = self.config.clone();
        let starting = std::thread::spawn(move || {
            let started = runtime.start(config);
            (runtime, started)
        });
        self.starting = Some((gate_tx, starting));
    }

    /// Lets the held start go on: the services answer, or they do not.
    pub(crate) fn finish_start(&mut self, services_answer: bool) -> Result<()> {
        let (gate, starting) = self.starting.take().context("no start is held")?;
        gate.send(services_answer)?;
        let (runtime, started) = starting
            .join()
            .map_err(|_| anyhow::anyhow!("the start panicked"))?;
        self.runtime = Some(runtime);
        self.started = Some(started);
        Ok(())
    }

    pub(crate) fn runtime(&self) -> Result<&Runtime> {
        self.runtime.as_ref().context("the runtime was not started")
    }

    pub(crate) fn icon_history(&self) -> Vec<(Instant, IconState)> {
        self.drawn.icons.lock().unwrap().clone()
    }

    fn icons_since(&self, mark: usize) -> Vec<IconState> {
        self.icon_history()[mark..]
            .iter()
            .map(|(_, icon)| *icon)
            .collect()
    }

    pub(crate) fn mark(&self) -> usize {
        self.drawn.icons.lock().unwrap().len()
    }

    pub(crate) fn current_icon(&self) -> Option<IconState> {
        self.icon_history().last().map(|(_, icon)| *icon)
    }

    fn icon_becomes(&self, wanted: impl Fn(IconState) -> bool) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        loop {
            if self.current_icon().is_some_and(&wanted) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "the icon is {:?}; drawn: {:?}",
                    self.current_icon(),
                    self.icon_history()
                );
            }
            std::thread::sleep(POLL);
        }
    }

    /// Waits until the icons drawn since the scenario's action are `expected`.
    pub(crate) fn drawn_since_becomes(&self, expected: &[IconState]) -> Result<()> {
        let deadline = Instant::now() + REACTION;
        loop {
            let drawn = self.icons_since(self.drawn.before);
            if drawn == expected {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!("drawn since the action: {drawn:?}");
            }
            std::thread::sleep(POLL);
        }
    }

    pub(crate) fn icon_is(&self, wanted: IconState) -> Result<()> {
        self.icon_becomes(|icon| icon == wanted)
    }

    pub(crate) fn send(&self, input: PipelineInput) -> Result<()> {
        self.parts
            .inputs
            .lock()
            .unwrap()
            .as_ref()
            .context("no pipeline runs")?
            .send(input)?;
        Ok(())
    }

    /// Presses the hotkey: a recording starts in Toggle mode, or stops when one runs.
    pub(crate) fn press(&self) {
        self.keyboard.signal(PttSignal::Pressed);
        self.keyboard.signal(PttSignal::Released);
    }

    pub(crate) fn start_recording(&self) -> Result<()> {
        match self.config.hotkey.recording_mode {
            RecordingMode::Toggle => self.press(),
            RecordingMode::Hold => self.keyboard.signal(PttSignal::Pressed),
        }
        self.icon_is(IconState::Recording)
    }

    pub(crate) fn open_menu(&mut self) {
        self.menu = Some(self.cell.menu());
    }

    pub(crate) fn status_line(&self) -> Result<&str> {
        Ok(&self
            .menu
            .as_ref()
            .context("the menu was not opened")?
            .status_line)
    }
}

/// Notes when the runtime stops, so a quit shows it stopped before the process ended.
pub(super) struct QuitObserver(Arc<Mutex<Vec<QuitEvent>>>);

impl RuntimeObserver for QuitObserver {
    fn state_changed(&self, state: &RuntimeState) {
        if *state == RuntimeState::Stopped {
            self.0.lock().unwrap().push(QuitEvent::RuntimeStopped);
        }
    }
}

pub(super) fn run(world: &mut IntegrationWorld) -> &mut MenuBarRun {
    world.app.menu_bar.get_or_insert_with(MenuBarRun::default)
}

pub(super) fn set(flag: &Flag, value: bool) {
    *flag.lock().unwrap() = value;
}

pub(super) fn recording_mode(name: &str) -> Result<RecordingMode> {
    match name {
        "hold" => Ok(RecordingMode::Hold),
        "toggle" => Ok(RecordingMode::Toggle),
        other => bail!("no recording mode {other:?}"),
    }
}
