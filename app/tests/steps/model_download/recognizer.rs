//! The voice control models the check lists for each command recognizer.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then};

use voicepuncher::core::domain::{CommandRecognizer, ModelUse, CATALOG};

use super::files::files;
use crate::IntegrationWorld;

const VOICE_GATE_MODEL: &str = "silero_vad.onnx";

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
