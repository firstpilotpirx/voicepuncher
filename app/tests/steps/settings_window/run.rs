//! The settings run: the settings form over a temporary configuration file.

use super::{
    display, ensure, find, ApplyStatus, Arc, Config, ConfigApplier, Context, Duration, Field,
    InputDevices, Instant, IntegrationWorld, Mutex, PathBuf, Refusal, Result, SettingsSession,
    StatusListener, Value, REACTION,
};

/// What a settings scenario shares between its steps.
pub struct SettingsRun {
    pub(crate) folder: tempfile::TempDir,
    pub(crate) original: Option<String>,
    pub(crate) microphones: Vec<String>,
    pub(crate) command_line: Vec<fn(&mut Config)>,
    pub(crate) session: Option<SettingsSession>,
    pub(crate) statuses: Arc<Mutex<Vec<ApplyStatus>>>,
    /// Counts the app's pastes, when the scenario runs the app.
    pub(crate) pastes: Option<Arc<dyn Fn() -> usize + Send + Sync>>,
    /// The pastes the app had made when the settings were said to be applied.
    pub(crate) pastes_when_applied: Arc<Mutex<Option<usize>>>,
    pub(crate) refusal: Option<Refusal>,
}

impl std::fmt::Debug for SettingsRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsRun")
            .field("statuses", &self.statuses)
            .field("refusal", &self.refusal)
            .finish_non_exhaustive()
    }
}

impl Default for SettingsRun {
    fn default() -> Self {
        Self {
            folder: tempfile::tempdir().expect("a temporary folder"),
            original: None,
            microphones: Vec::new(),
            command_line: Vec::new(),
            session: None,
            statuses: Arc::default(),
            pastes: None,
            pastes_when_applied: Arc::default(),
            refusal: None,
        }
    }
}

impl SettingsRun {
    pub(crate) fn path(&self) -> PathBuf {
        self.folder.path().join("config.toml")
    }

    pub(crate) fn write(&mut self, text: &str) {
        std::fs::write(self.path(), text).expect("the configuration file is written");
        self.original = Some(text.to_owned());
    }

    pub(crate) fn file(&self) -> Result<String> {
        std::fs::read_to_string(self.path()).context("the configuration file is not there")
    }

    pub(crate) fn session(&mut self) -> Result<&mut SettingsSession> {
        self.session
            .as_mut()
            .context("the settings were not opened")
    }

    pub(crate) fn shown(&mut self, section: &str, key: &str) -> Result<String> {
        let field = field(section, key);
        Ok(display(field, &self.session()?.value(field)))
    }

    pub(crate) fn edit(&mut self, section: &str, key: &str, value: Value) -> Result<()> {
        self.session()?.edit(field(section, key), value);
        Ok(())
    }

    pub(crate) fn save(&mut self) -> Result<()> {
        let statuses = Arc::clone(&self.statuses);
        let pastes = self.pastes.clone();
        let when_applied = Arc::clone(&self.pastes_when_applied);
        let listener: StatusListener = Box::new(move |s| {
            if let (ApplyStatus::Applied, Some(pastes)) = (&s, &pastes) {
                *when_applied.lock().unwrap() = Some(pastes());
            }
            statuses.lock().unwrap().push(s);
        });
        self.refusal = self.session()?.save(listener).err();
        Ok(())
    }

    pub(crate) fn wait_for_status(
        &self,
        wanted: impl Fn(&ApplyStatus) -> bool,
    ) -> Result<ApplyStatus> {
        let deadline = Instant::now() + REACTION;
        loop {
            if let Some(status) = self.statuses.lock().unwrap().iter().find(|s| wanted(s)) {
                return Ok(status.clone());
            }
            ensure!(Instant::now() < deadline, "statuses: {:?}", self.statuses);
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub(crate) fn marked(&self) -> Result<String> {
        let refusal = self.refusal.as_ref().context("the Save was not refused")?;
        Ok(refusal.field.context("no field is marked")?.path())
    }
}

pub(super) fn field(section: &str, key: &str) -> &'static Field {
    find(section, key).unwrap_or_else(|| panic!("no field for {section}.{key}"))
}

pub(super) fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

pub(super) fn settings(world: &mut IntegrationWorld) -> &mut SettingsRun {
    world.app.settings.get_or_insert_with(SettingsRun::default)
}

/// Saved settings with no app running: applied at once.
pub(super) struct NoApp;

impl ConfigApplier for NoApp {
    fn apply(&self, _config: Config, mut listener: StatusListener) {
        listener(ApplyStatus::Applied);
    }
}

pub(super) struct FakeMicrophones(pub(crate) Vec<String>);

impl InputDevices for FakeMicrophones {
    fn names(&self) -> Vec<String> {
        self.0.clone()
    }
}
