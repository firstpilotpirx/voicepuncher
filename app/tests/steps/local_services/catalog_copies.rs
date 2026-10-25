//! How many copies of the command spotting service the catalog starts, and on which ports
//! the copies of each service lie.

use std::collections::HashSet;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::config::Config;
use voicepuncher::core::domain::ServiceKind;
use voicepuncher_copy_pool::{
    ServiceAddress, COMMAND_SPOTTING_COPY_OFFSET, SPEECH_RECOGNITION_COPY_OFFSET,
};

use super::catalog::{catalog, CatalogRun};
use crate::IntegrationWorld;

/// The port of the command spotting service on its default address.
const COMMAND_SPOTTING_PORT: u16 = 8082;

#[given("voice control is on and the command spotting service is configured as one copy")]
fn one_spotting_copy(world: &mut IntegrationWorld) -> Result<()> {
    catalog(world)?.config.services.command_spotting_copies = 1;
    Ok(())
}

#[given("every service on its default address")]
fn default_addresses(world: &mut IntegrationWorld) -> Result<()> {
    catalog(world).map(|_| ())
}

#[when("the copy ports of each service are worked out")]
fn copy_ports_worked_out(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let services = [
        (
            run.config.transcription.base_url.clone(),
            SPEECH_RECOGNITION_COPY_OFFSET,
        ),
        (
            run.config.services.commands_url.clone(),
            SPEECH_RECOGNITION_COPY_OFFSET,
        ),
        (
            run.config.voice_control.url.clone(),
            COMMAND_SPOTTING_COPY_OFFSET,
        ),
    ];
    run.copy_ports = services
        .iter()
        .map(|(url, offset)| {
            Ok(ServiceAddress::parse(url)?
                .with_copy_offset(*offset)
                .copy_ports())
        })
        .collect::<Result<_>>()?;
    Ok(())
}

fn spotting_ports(run: &CatalogRun) -> Vec<u16> {
    run.of_kind(ServiceKind::CommandSpotting)
        .iter()
        .map(|spec| spec.port)
        .collect()
}

#[then("the command spotting service is started as two copies on its copy ports, and nothing on its own port")]
fn two_spotting_copies(world: &mut IntegrationWorld) -> Result<()> {
    let ports = spotting_ports(catalog(world)?);
    let copy_ports = ServiceAddress::parse(&Config::default().voice_control.url)?
        .with_copy_offset(COMMAND_SPOTTING_COPY_OFFSET)
        .copy_ports();
    ensure!(ports.len() == 2, "{} copies on {ports:?}", ports.len());
    ensure!(
        ports.iter().all(|port| copy_ports.contains(port)),
        "{ports:?} not of {copy_ports:?}"
    );
    ensure!(
        !ports.contains(&COMMAND_SPOTTING_PORT),
        "a copy on the service's own port"
    );
    Ok(())
}

#[then("the command spotting service is started once, on its own port")]
fn one_spotting_copy_on_own_port(world: &mut IntegrationWorld) -> Result<()> {
    let ports = spotting_ports(catalog(world)?);
    ensure!(ports == [COMMAND_SPOTTING_PORT], "started on {ports:?}");
    Ok(())
}

#[then("no port is a copy port of two services")]
fn copy_ports_apart(world: &mut IntegrationWorld) -> Result<()> {
    let run = catalog(world)?;
    let all: Vec<u16> = run.copy_ports.iter().flatten().copied().collect();
    let distinct: HashSet<u16> = all.iter().copied().collect();
    ensure!(!all.is_empty(), "no copy port was worked out");
    ensure!(
        distinct.len() == all.len(),
        "shared copy ports: {:?}",
        run.copy_ports
    );
    Ok(())
}
