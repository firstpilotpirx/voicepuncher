//! Steps of the dictation service's model: a chunk without speech and the model it runs.

use super::{bail, given, off_thread, then, when, Context, E2eWorld, Result, ScenarioServices};
use crate::steps::local_services::transcribe;

/// 2.3 s at 16 kHz: a command-length chunk, and the length that crashed whisper-server 1.9.5
/// when it computed language probabilities for audio without speech.
const SILENT_CHUNK_SAMPLES: usize = 36_800;

/// The words the service hears in a chunk without speech, or why it gave no answer.
fn transcribe_silence(url: &str) -> std::result::Result<usize, String> {
    transcribe(url, vec![0; SILENT_CHUNK_SAMPLES])
        .map(|text| text.split_whitespace().count())
        .map_err(|e| format!("{e:#}"))
}

#[given("the service scripts started the speech recognition service for dictation")]
pub(super) async fn dictation_service_started(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        world.services.local_services = Some(ScenarioServices::with_dictation_service()?);
        Ok(())
    })
    .await
}

#[when("a chunk that holds no speech is sent to it")]
pub(super) async fn chunk_without_speech(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = world
            .services
            .local_services
            .as_mut()
            .context("the speech recognition service was not started by a Given step")?;
        services.heard_words = Some(transcribe_silence(&services.transcription_url()));
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
            .context("no chunk was sent")?
        {
            Ok(0) => {}
            Ok(words) => bail!("{words} words were heard in a chunk without speech"),
            Err(e) => bail!("the transcription of a chunk without speech failed: {e}"),
        }
        if let Err(e) = transcribe_silence(&services.transcription_url()) {
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
        scenario_services(world)?.check_runs_default_model()
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
