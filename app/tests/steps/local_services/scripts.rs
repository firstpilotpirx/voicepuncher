//! The services left to the service scripts: a configuration that says the app does not
//! manage them, and the runtime launched with it.

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::config::{parse, Config};
use voicepuncher::runtime::RuntimeState;

use super::local;
use crate::steps::app_runtime;
use crate::IntegrationWorld;

const LEFT_TO_SCRIPTS: &str = "[services]\nmanage = false\n\n[voice_control]\nenabled = false\n";

#[given("the configuration says the app does not manage its services")]
fn not_managed(world: &mut IntegrationWorld) -> Result<()> {
    local(world).scripts_config = Some(parse(LEFT_TO_SCRIPTS, "config.toml")?);
    Ok(())
}

#[when("the app launches with that configuration")]
fn app_launches(world: &mut IntegrationWorld) -> Result<()> {
    let config: Config = local(world)
        .scripts_config
        .clone()
        .context("no configuration")?;
    // a configuration that leaves the services to the scripts gives the runtime no services
    // to start: it connects to the ones that answer
    app_runtime::run(world).start_from(config)
}

#[then("the app starts no service and connects to the services that answer")]
fn starts_none(world: &mut IntegrationWorld) -> Result<()> {
    let config = local(world)
        .scripts_config
        .clone()
        .context("no configuration")?;
    ensure!(!config.services.manage, "the app manages its services");
    app_runtime::run(world).state_becomes(|state| matches!(state, RuntimeState::Ready { .. }))
}
