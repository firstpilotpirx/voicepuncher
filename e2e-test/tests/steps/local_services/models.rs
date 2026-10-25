//! Steps of `local_services.feature` about the model each speech recognition service runs, and
//! what the steps of `speech_recognition.feature` about the dictation model use: a home of the
//! scenario's own holds the models the service scripts download, and each service's command
//! line and log show the model it loaded.

use std::ffi::OsStr;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use regex::Regex;

use voicepuncher_e2e_test::voice_commands::{
    recognized_commands, spoken_with_pauses, spotting_events,
};

use super::confirmations::{log_len, log_since_speech, ANSWERED, GIVEN_UP};
use super::copies::{client_requests, short_speech, transcribe};
use super::model_home::{link_models, COMMAND_MODEL, DICTATION_MODEL};
use super::{
    service_process, services, ScenarioServices, COMMAND_SPEECH_RECOGNITION, SPEECH_RECOGNITION,
};
use crate::steps::off_thread;
use crate::E2eWorld;

/// A model the user names, under a name the scripts never look for.
const NAMED_MODEL: &str = "whisper-named-by-the-user.bin";

/// One of the two speech recognition services.
#[derive(Debug, Clone, Copy)]
pub enum Role {
    Dictation,
    Commands,
}

impl Role {
    /// The service's name in the service scripts.
    fn name(self) -> &'static str {
        match self {
            Self::Dictation => SPEECH_RECOGNITION,
            Self::Commands => COMMAND_SPEECH_RECOGNITION,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Dictation => "speech recognition service for dictation",
            Self::Commands => "speech recognition service for commands",
        }
    }
}

impl ScenarioServices {
    /// Services on free ports, one copy of each speech recognition service, in a home of their
    /// own whose `.voicepuncher/models/` links the downloaded models under their download
    /// names beside a larger file that is no model.
    fn in_model_home() -> Result<Self> {
        let mut services = Self::with_copies(Some(1))?;
        services.options.command_copies = Some(1);
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

    /// Starts both speech recognition services with the models the scripts pick by default.
    pub fn start_without_naming_models(&mut self) -> Result<()> {
        self.options.default_speech_model = true;
        self.options.default_command_model = true;
        self.start_each(&[SPEECH_RECOGNITION, COMMAND_SPEECH_RECOGNITION])
    }

    /// Starts both speech recognition services, naming for dictation a model under a name the
    /// scripts never look for, and the commands model the scripts pick by default.
    pub fn start_naming_a_dictation_model(&mut self) -> Result<()> {
        let named_dir = self
            .home
            .as_ref()
            .context("the scenario has no home of its own")?
            .path()
            .join("named");
        std::fs::create_dir_all(&named_dir)?;
        let named = named_dir.join(NAMED_MODEL);
        std::os::unix::fs::symlink(self.default_model_in_home(COMMAND_MODEL)?, &named)?;
        self.options.speech_model = Some(named);
        self.options.default_command_model = true;
        self.start_each(&[SPEECH_RECOGNITION, COMMAND_SPEECH_RECOGNITION])
    }

    /// Fails unless the speech recognition service of `role` runs the model the scripts
    /// download for it, as linked in the scenario's home.
    pub fn check_runs_default_model(&self, role: Role) -> Result<()> {
        let file = match role {
            Role::Dictation => DICTATION_MODEL,
            Role::Commands => COMMAND_MODEL,
        };
        self.check_loads(role, &self.default_model_in_home(file)?)
    }

    /// Fails unless the speech recognition service for dictation runs the model the user named.
    pub fn check_runs_named_dictation_model(&self) -> Result<()> {
        let model = self
            .options
            .speech_model
            .clone()
            .context("no model was named")?;
        self.check_loads(Role::Dictation, &model)
    }

    fn port_of(&self, role: Role) -> u16 {
        match role {
            Role::Dictation => self.stt_port,
            Role::Commands => self.command_stt_port,
        }
    }

    /// The log of the speech recognition service of `role`, run as one copy.
    fn speech_log(&self, role: Role) -> PathBuf {
        self.dir("logs")
            .join(format!("{SPEECH_RECOGNITION}-{}.log", self.port_of(role)))
    }

    /// Fails unless the speech recognition service of `role` started, runs `model` (its command
    /// line and its log say so) and transcribes a short utterance.
    fn check_loads(&self, role: Role, model: &Path) -> Result<()> {
        let label = role.label();
        let log = std::fs::read_to_string(self.speech_log(role)).unwrap_or_default();
        let (_, output) = self
            .run
            .starts
            .iter()
            .find(|(name, _)| name == role.name())
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
        let port = self.port_of(role);
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

fn model_scenario(world: &mut E2eWorld) -> Result<&mut ScenarioServices> {
    services(world)
}

#[given("the models that the service scripts download are on disk")]
fn downloaded_models_on_disk(world: &mut E2eWorld) -> Result<()> {
    world.services.local_services = Some(ScenarioServices::in_model_home()?);
    Ok(())
}

/// Every service, by one start of the service scripts that names none, in the user's home,
/// where the command spotting service is built: one copy of each speech recognition service,
/// with the commands model the scripts pick by default.
#[given("the user started all services with the service scripts")]
async fn all_services_started(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = model_scenario(world)?;
        services.home = None;
        services.options.default_command_model = true;
        services.start(&[])
    })
    .await
}

#[when(expr = "an app streams {string} to the command spotting service")]
async fn app_streams(world: &mut E2eWorld, phrase: String) -> Result<()> {
    off_thread(world, move |world| {
        let services = model_scenario(world)?;
        services.watched.log_before_speech = Some(log_len(services)?);
        let audio = spoken_with_pauses(&phrase)?;
        let events = spotting_events(
            &services.command_spotting_url(),
            &audio,
            &VoiceCommand::STANDBY,
        )?;
        world.voice.recognized = recognized_commands(events);
        Ok(())
    })
    .await
}

#[then("the speech recognition service for commands answered its confirmation with the small model, and the speech recognition service for dictation got no request")]
fn confirmed_by_commands_service(world: &mut E2eWorld) -> Result<()> {
    let services = model_scenario(world)?;
    let dictation_log =
        std::fs::read_to_string(services.speech_log(Role::Dictation)).unwrap_or_default();
    let dictation_requests = client_requests(&dictation_log).count();
    ensure!(
        dictation_requests == 0,
        "the speech recognition service for dictation answered {dictation_requests} confirmation request(s)"
    );
    let log = std::fs::read_to_string(services.speech_log(Role::Commands)).unwrap_or_default();
    let loaded = loaded_model(&log)
        .context("the speech recognition service for commands names no model it loaded")?;
    ensure!(
        Path::new(&loaded).file_name() == Some(OsStr::new(COMMAND_MODEL)),
        "the speech recognition service for commands loaded {loaded}, not {COMMAND_MODEL}"
    );
    ensure!(
        client_requests(&log).next().is_some(),
        "the speech recognition service for commands got no confirmation request"
    );
    confirmations_answered(services)
}

/// Fails unless the command spotting service's log since the speech shows a confirmation
/// answered and none that waited out its deadline: a speech recognition service that takes a
/// request and never answers it leaves only a given-up confirmation there.
fn confirmations_answered(services: &ScenarioServices) -> Result<()> {
    let spotting_log = log_since_speech(services)?;
    ensure!(
        spotting_log.contains(ANSWERED),
        "the command spotting service logged no answered confirmation:\n{spotting_log}"
    );
    ensure!(
        !spotting_log.contains(GIVEN_UP),
        "a confirmation of the command spotting service waited out its deadline:\n{spotting_log}"
    );
    Ok(())
}
