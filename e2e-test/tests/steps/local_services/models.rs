//! What the steps of `speech_recognition.feature` about the dictation model use: a home of the
//! scenario's own holds the models the service scripts download, and the service's command
//! line and log show the model it loaded.

use voicepuncher::test_doubles::prelude::*;

use cucumber::given;
use regex::Regex;

use super::copies::{short_speech, transcribe};
use super::model_home::{link_models, DICTATION_MODEL};
use super::{service_process, ScenarioServices, SPEECH_RECOGNITION};
use crate::E2eWorld;

/// A model the user names, under a name the scripts never look for.
const NAMED_MODEL: &str = "whisper-named-by-the-user.bin";

/// How the service scripts name the service for dictation in a failure.
const LABEL: &str = "speech recognition service for dictation";

impl ScenarioServices {
    /// Services on free ports, the speech recognition service for dictation as one copy, in a
    /// home of their own whose `.voicepuncher/models/` links the downloaded models under their
    /// download names beside a larger file that is no model.
    fn in_model_home() -> Result<Self> {
        let mut services = Self::with_copies(Some(1))?;
        let home = tempfile::tempdir()?;
        link_models(&home.path().join(".voicepuncher").join("models"))?;
        services.home = Some(home);
        Ok(services)
    }

    /// The path a model the scripts download has in the scenario's home.
    fn default_model_in_home(&self, file: &str) -> Result<PathBuf> {
        Ok(self
            .home
            .as_ref()
            .context("the scenario has no home of its own")?
            .path()
            .join(".voicepuncher")
            .join("models")
            .join(file))
    }

    /// Starts each named service with the service scripts, one call per service, keeping what
    /// each start printed; a start that fails does not stop the others.
    pub(super) fn start_each(&mut self, names: &[&str]) -> Result<()> {
        for name in names {
            let output = self.just(&["start-model-servers", name])?;
            self.run.starts.push(((*name).to_owned(), output));
        }
        Ok(())
    }

    /// Starts the speech recognition service for dictation with the model the scripts pick by
    /// default.
    pub fn start_without_naming_models(&mut self) -> Result<()> {
        self.options.default_speech_model = true;
        self.start_each(&[SPEECH_RECOGNITION])
    }

    /// Starts the speech recognition service for dictation, naming the downloaded model under
    /// a name the scripts never look for.
    pub fn start_naming_a_dictation_model(&mut self) -> Result<()> {
        let named_dir = self
            .home
            .as_ref()
            .context("the scenario has no home of its own")?
            .path()
            .join("named");
        std::fs::create_dir_all(&named_dir)?;
        let named = named_dir.join(NAMED_MODEL);
        std::os::unix::fs::symlink(self.default_model_in_home(DICTATION_MODEL)?, &named)?;
        self.options.speech_model = Some(named);
        self.start_each(&[SPEECH_RECOGNITION])
    }

    /// Fails unless the speech recognition service for dictation runs the model the scripts
    /// download for it, as linked in the scenario's home.
    pub fn check_runs_default_model(&self) -> Result<()> {
        self.check_loads(&self.default_model_in_home(DICTATION_MODEL)?)
    }

    /// Fails unless the speech recognition service for dictation runs the model the user named.
    pub fn check_runs_named_dictation_model(&self) -> Result<()> {
        let model = self
            .options
            .speech_model
            .clone()
            .context("no model was named")?;
        self.check_loads(&model)
    }

    /// The log of the speech recognition service for dictation, run as one copy.
    fn speech_log(&self) -> PathBuf {
        self.dir("logs")
            .join(format!("{SPEECH_RECOGNITION}-{}.log", self.stt_port))
    }

    /// Fails unless the speech recognition service for dictation started, runs `model` (its
    /// command line and its log say so) and transcribes a short utterance.
    fn check_loads(&self, model: &Path) -> Result<()> {
        let label = LABEL;
        let log = std::fs::read_to_string(self.speech_log()).unwrap_or_default();
        let (_, output) = self
            .run
            .starts
            .iter()
            .find(|(name, _)| name == SPEECH_RECOGNITION)
            .with_context(|| format!("the {label} was never started"))?;
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            let loaded = loaded_model(&log)
                .or_else(|| loaded_model(&said))
                .map(|m| format!(" on {m}"))
                .unwrap_or_default();
            bail!(
                "the {label} did not start{loaded}, not on {}:\n{said}",
                model.display()
            );
        }
        let port = self.stt_port;
        check_runs(label, port, model, &log)?;
        let heard = transcribe(&format!("http://127.0.0.1:{port}/v1"), short_speech()?)?;
        ensure!(
            !heard.trim().is_empty(),
            "the {label} heard nothing in a short utterance"
        );
        Ok(())
    }
}

/// Fails unless the process of the service on `port` runs `model` and its `log` says it loaded it.
fn check_runs(label: &str, port: u16, model: &Path, log: &str) -> Result<()> {
    let pid = service_process(port)
        .with_context(|| format!("no process of the {label} on port {port}"))?;
    let command_line = command_line(pid)?;
    ensure!(
        command_line.contains(&format!("-m {} ", model.display())),
        "the {label} runs {command_line:?}, not on {}",
        model.display()
    );
    let loaded = loaded_model(log)
        .with_context(|| format!("the {label}'s log names no model it loaded:\n{log}"))?;
    ensure!(
        Path::new(&loaded) == model,
        "the {label} loaded {loaded}, not {}",
        model.display()
    );
    Ok(())
}

/// The model whisper-server says it loads in `text`, its log or the scripts' report.
fn loaded_model(text: &str) -> Option<String> {
    Regex::new(r"loading model from '([^']+)'")
        .expect("valid pattern")
        .captures_iter(text)
        .last()
        .map(|c| c[1].to_owned())
}

fn command_line(pid: u32) -> Result<String> {
    let output = Command::new("ps")
        .args(["-o", "command=", "-p", &pid.to_string()])
        .output()?;
    Ok(format!(
        "{} ",
        String::from_utf8_lossy(&output.stdout).trim()
    ))
}

#[given("the models that the service scripts download are on disk")]
fn downloaded_models_on_disk(world: &mut E2eWorld) -> Result<()> {
    world.services.local_services = Some(ScenarioServices::in_model_home()?);
    Ok(())
}
