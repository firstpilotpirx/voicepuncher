//! The command history the catalog tells the command spotting copies it starts to keep: the
//! voice-control scenarios of the app's own services, looked up on a home of their own.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then};

use voicepuncher::core::domain::ServiceKind;

use super::catalog::catalog;
use crate::IntegrationWorld;

const HISTORY_FLAG: &str = "--command-history-dir";

#[given("the default configuration with voice control on")]
fn default_with_voice_control(world: &mut IntegrationWorld) -> Result<()> {
    catalog(world)?;
    Ok(())
}

#[given("voice control is on and the user turned the command history off")]
fn history_off(world: &mut IntegrationWorld) -> Result<()> {
    catalog(world)?.config.archive.keep_commands = false;
    Ok(())
}

#[then("each copy of the command spotting service is told to keep the command history in a folder under the app's home folder")]
fn told_to_keep_history(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let home = run.home().display().to_string();
    let copies = run.of_kind(ServiceKind::CommandSpotting);
    ensure!(!copies.is_empty(), "no command spotting copy is started");
    for spec in copies {
        let dir = spec.args.iter().skip_while(|a| *a != HISTORY_FLAG).nth(1);
        ensure!(
            dir.is_some_and(|d| d.starts_with(&home)),
            "{} keeps no command history under {home}: {:?}",
            spec.label,
            spec.args
        );
    }
    Ok(())
}

#[then("no copy of the command spotting service is told to keep a command history")]
fn told_to_keep_none(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let copies = run.of_kind(ServiceKind::CommandSpotting);
    ensure!(!copies.is_empty(), "no command spotting copy is started");
    for spec in copies {
        ensure!(
            !spec.args.iter().any(|a| a == HISTORY_FLAG),
            "{} keeps a command history: {:?}",
            spec.label,
            spec.args
        );
    }
    Ok(())
}
