//! Steps of the services' models: a silent utterance and the models each service runs.

use super::{
    bail, given, off_thread, then, when, Context, E2eWorld, Result, Role, ScenarioServices,
};

/// 2.3 s at 16 kHz: a command-length utterance, and the length that crashed whisper-server
/// 1.9.5 when it computed language probabilities for audio without speech.
pub(super) const SILENT_UTTERANCE_SAMPLES: usize = 36_800;

pub(super) const UTTERANCE_SAMPLE_RATE: u32 = 16_000;

/// As long as the command spotting service waited for a confirmation before it had a deadline.
pub(super) const UTTERANCE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

pub(super) fn confirm_silence(url: &str) -> std::result::Result<usize, String> {
    voicepuncher_command_spotting::confirm::UtteranceTranscriber::new(url)
        .map_err(|e| e.to_string())?
        .transcribe(
            &vec![0.0; SILENT_UTTERANCE_SAMPLES],
            UTTERANCE_SAMPLE_RATE,
            UTTERANCE_DEADLINE,
        )
        .map(|heard| heard.words.len())
        .map_err(|e| format!("{e:#}"))
}

#[given("the service scripts started the speech recognition service for commands")]
pub(super) async fn command_speech_recognition_started(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        world.services.local_services = Some(ScenarioServices::with_command_service()?);
        Ok(())
    })
    .await
}

#[when("the command spotting service asks it to confirm an utterance that holds no speech")]
pub(super) async fn confirm_utterance_without_speech(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = world
            .services
            .local_services
            .as_mut()
            .context("the speech recognition service was not started by a Given step")?;
        services.heard_words = Some(confirm_silence(&services.command_transcription_url()));
        Ok(())
    })
    .await
}

#[then("no words are heard and the service answers the next request")]
pub(super) async fn no_words_and_service_answers(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = world
            .services
            .local_services
            .as_ref()
            .context("the speech recognition service was not started by a Given step")?;
        match services
            .heard_words
            .as_ref()
            .context("nothing was confirmed")?
        {
            Ok(0) => {}
            Ok(words) => bail!("{words} words were heard in an utterance without speech"),
            Err(e) => bail!("the confirmation of an utterance without speech failed: {e}"),
        }
        if let Err(e) = confirm_silence(&services.command_transcription_url()) {
            bail!("the service does not answer the next request: {e}");
        }
        Ok(())
    })
    .await
}

pub(super) fn scenario_services(world: &mut E2eWorld) -> Result<&mut ScenarioServices> {
    world
        .services
        .local_services
        .as_mut()
        .context("no services were set up by a Given step")
}

#[when("the user starts the services with the service scripts without naming a model")]
pub(super) async fn start_without_naming_a_model(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        scenario_services(world)?.start_without_naming_models()
    })
    .await
}

#[when("the user starts the services with the service scripts and names another speech recognition model for dictation")]
pub(super) async fn start_naming_a_dictation_model(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        scenario_services(world)?.start_naming_a_dictation_model()
    })
    .await
}

#[then(
    "the speech recognition service for dictation recognizes speech with the large-v3-turbo model"
)]
pub(super) async fn dictation_on_turbo(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        scenario_services(world)?.check_runs_default_model(Role::Dictation)
    })
    .await
}

#[then("the speech recognition service for dictation recognizes speech with the named model")]
pub(super) async fn dictation_on_named_model(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        scenario_services(world)?.check_runs_named_dictation_model()
    })
    .await
}

#[then("the speech recognition service for commands keeps the small model")]
pub(super) async fn commands_on_small(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        scenario_services(world)?.check_runs_default_model(Role::Commands)
    })
    .await
}
