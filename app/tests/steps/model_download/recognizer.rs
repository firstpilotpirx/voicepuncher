//! The voice control models the check lists for each command recognizer.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then};

use voicepuncher::core::domain::{CommandRecognizer, ModelUse, CATALOG};

use super::files::files;
use crate::IntegrationWorld;

const VOICE_GATE_MODEL: &str = "silero_vad.onnx";
const WHISPER_SMALL_MODEL: &str = "ggml-small.bin";

/// The catalog files only `used_by` needs.
fn used_by(used_by: ModelUse) -> Vec<&'static str> {
    CATALOG
        .iter()
        .filter(|file| file.used_by == used_by)
        .map(|file| file.name)
        .collect()
}

/// Fails unless every one of `names` and the voice gate model was listed as missing.
fn all_listed(world: &mut IntegrationWorld, names: &[&'static str]) -> Result<()> {
    let listed = files(world).listed_missing()?;
    for name in names.iter().chain([&VOICE_GATE_MODEL]) {
        ensure!(listed.contains(name), "{name} is not listed: {listed:?}");
    }
    Ok(())
}

/// Fails when a file only `use` needs was listed.
fn none_listed(world: &mut IntegrationWorld, used: ModelUse) -> Result<()> {
    let listed = files(world).listed_missing()?;
    let unwanted: Vec<_> = used_by(used)
        .into_iter()
        .filter(|name| listed.contains(name))
        .collect();
    ensure!(unwanted.is_empty(), "listed: {unwanted:?}");
    Ok(())
}

#[given("voice control is on with the streaming recognizer and no voice control model is on disk")]
fn streaming_without_models(world: &mut IntegrationWorld) {
    files(world).voice_control = Some(CommandRecognizer::Streaming);
}

#[given("voice control is on with the keyword spotter and no voice control model is on disk")]
fn keyword_spotter_without_models(world: &mut IntegrationWorld) {
    files(world).voice_control = Some(CommandRecognizer::KeywordSpotting);
}

#[then(
    "the English and the Russian streaming models and the voice gate model are listed as Missing"
)]
fn streaming_models_missing(world: &mut IntegrationWorld) -> Result<()> {
    let streaming = used_by(ModelUse::Streaming);
    ensure!(
        streaming.len() == 2,
        "streaming models of the catalog: {streaming:?}"
    );
    all_listed(world, &streaming)
}

#[then("neither the Whisper small model nor the keyword spotting model is listed")]
fn keyword_spotting_models_not_listed(world: &mut IntegrationWorld) -> Result<()> {
    none_listed(world, ModelUse::KeywordSpotting)
}

#[then("the Whisper small model, the keyword spotting model and the voice gate model are listed as Missing")]
fn keyword_spotting_models_missing(world: &mut IntegrationWorld) -> Result<()> {
    let keyword_spotting = used_by(ModelUse::KeywordSpotting);
    ensure!(
        keyword_spotting.contains(&WHISPER_SMALL_MODEL) && keyword_spotting.len() == 2,
        "keyword spotting models of the catalog: {keyword_spotting:?}"
    );
    all_listed(world, &keyword_spotting)
}

#[then("no streaming model is listed")]
fn no_streaming_model_listed(world: &mut IntegrationWorld) -> Result<()> {
    none_listed(world, ModelUse::Streaming)
}
