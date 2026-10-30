//! The services the catalog starts for each command recognizer the configuration names.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then};

use voicepuncher::config::VoiceControlCfg;
use voicepuncher::core::domain::{CommandRecognizer, ServiceKind};

use super::catalog::catalog;
use super::home::{KWS_MODEL_DIR, STREAMING_EN_MODEL_DIR, STREAMING_RU_MODEL_DIR};
use crate::IntegrationWorld;

#[given("no local service runs, voice control is on and the user never chose a command recognizer")]
fn recognizer_never_chosen(world: &mut IntegrationWorld) -> Result<()> {
    catalog(world)?.config.voice_control.recognizer = VoiceControlCfg::default().recognizer;
    Ok(())
}

#[given("no local service runs, voice control is on and the keyword spotter is chosen as the command recognizer")]
fn keyword_spotter_chosen(world: &mut IntegrationWorld) -> Result<()> {
    catalog(world)?.config.voice_control.recognizer = CommandRecognizer::KeywordSpotting;
    Ok(())
}

#[then("the command spotting service is started with the streaming models")]
fn started_with_streaming_models(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let copies = run.of_kind(ServiceKind::CommandSpotting);
    ensure!(
        !copies.is_empty(),
        "no command spotting service was looked up"
    );
    for spec in copies {
        let args = spec.args.join(" ");
        ensure!(
            args.contains("--recognizer streaming"),
            "{}: {args}",
            spec.label
        );
        for dir in [STREAMING_EN_MODEL_DIR, STREAMING_RU_MODEL_DIR] {
            ensure!(
                args.contains(dir),
                "{} starts without {dir}: {args}",
                spec.label
            );
        }
        ensure!(!args.contains(KWS_MODEL_DIR), "{}: {args}", spec.label);
    }
    Ok(())
}

#[then("the speech recognition service for commands is not started")]
fn no_commands_service(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let commands = run.of_kind(ServiceKind::Commands);
    ensure!(commands.is_empty(), "started: {commands:?}");
    Ok(())
}

#[then("the speech recognition service for commands and the command spotting service with the keyword spotting model are started")]
fn keyword_spotting_services(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    ensure!(
        !run.of_kind(ServiceKind::Commands).is_empty(),
        "the speech recognition service for commands was not looked up"
    );
    let copies = run.of_kind(ServiceKind::CommandSpotting);
    ensure!(
        !copies.is_empty(),
        "no command spotting service was looked up"
    );
    for spec in copies {
        let args = spec.args.join(" ");
        ensure!(args.contains(KWS_MODEL_DIR), "{}: {args}", spec.label);
        ensure!(
            !args.contains("--recognizer streaming"),
            "{}: {args}",
            spec.label
        );
    }
    Ok(())
}
