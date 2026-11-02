//! The voice control models the check lists.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then};

use voicepuncher::core::domain::{ModelUse, CATALOG};

use super::files::files;
use crate::IntegrationWorld;

const VOICE_GATE_MODEL: &str = "silero_vad.onnx";
/// The models of the retired keyword spotter and its Whisper confirmation.
const RETIRED_MODELS: [&str; 2] = [
    "ggml-small.bin",
    "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2",
];

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

#[given("voice control is on with the streaming recognizer and no voice control model is on disk")]
fn streaming_without_models(world: &mut IntegrationWorld) {
    files(world).voice_control = true;
}

#[then(
    "the English and the Russian streaming models and the voice gate model are listed as Missing"
)]
fn streaming_models_missing(world: &mut IntegrationWorld) -> Result<()> {
    let streaming: Vec<_> = used_by(ModelUse::VoiceControl)
        .into_iter()
        .filter(|name| *name != VOICE_GATE_MODEL)
        .collect();
    ensure!(
        streaming.len() == 2,
        "streaming models of the catalog: {streaming:?}"
    );
    all_listed(world, &streaming)
}

#[then("neither the Whisper small model nor the keyword spotting model is listed")]
fn keyword_spotting_models_not_listed(world: &mut IntegrationWorld) -> Result<()> {
    let listed = files(world).listed_missing()?;
    let unwanted: Vec<_> = RETIRED_MODELS
        .into_iter()
        .filter(|name| listed.contains(name))
        .collect();
    ensure!(unwanted.is_empty(), "listed: {unwanted:?}");
    Ok(())
}
