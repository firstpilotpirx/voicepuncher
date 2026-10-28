//! The services the app manages, as the menu shows them: the status item's observer told the
//! runtime starts them, and the menu's actions over a view of services set by the scenario.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use voicepuncher::adapters::primary::menubar::macos::actions::{MacosMenuActions, MenuActions};
use voicepuncher::adapters::primary::menubar::macos::mapping::{offers_restart, service_line};

use voicepuncher::core::domain::{ServiceKind, ServiceState, ServiceStatus};
use voicepuncher::runtime::{RuntimeObserver, RuntimeState, ServicesView};

use super::run;
use crate::IntegrationWorld;

const DICTATION_COPIES: [u16; 3] = [18082, 18083, 18084];
const COMMANDS: u16 = 8085;
const COMMAND_SPOTTING: u16 = 8082;

/// The services the scenario sets and the restarts the menu asked for.
#[derive(Default)]
pub struct ViewDouble {
    statuses: Mutex<Vec<ServiceStatus>>,
    restarted: Mutex<Vec<u16>>,
}

impl ServicesView for ViewDouble {
    fn statuses(&self) -> Vec<ServiceStatus> {
        self.statuses.lock().unwrap().clone()
    }

    fn restart(&self, port: u16) {
        self.restarted.lock().unwrap().push(port);
    }

    fn recheck(&self, _port: u16) {}
}

/// The Services submenu: each service's line and whether it offers Restart.
pub type ServiceMenu = Vec<(u16, String, bool)>;

fn status(
    kind: ServiceKind,
    label: String,
    port: u16,
    state: ServiceState,
    owned: bool,
) -> ServiceStatus {
    ServiceStatus {
        kind,
        label,
        port,
        state,
        owned,
        log_file: PathBuf::from(format!("/logs/{port}.log")),
    }
}

fn with_services(world: &mut IntegrationWorld, statuses: Vec<ServiceStatus>) {
    let run = run(world);
    let view = Arc::new(ViewDouble::default());
    *view.statuses.lock().unwrap() = statuses;
    let scratch = std::env::temp_dir();
    let actions = MacosMenuActions::new(
        run.runtime_parts(None),
        Box::new(super::SettingsOpens(Arc::default())),
        scratch.join("logs"),
    )
    .with_services(Arc::clone(&view) as Arc<dyn ServicesView>);
    run.actions = Some(actions);
    run.services.view = Some(view);
}

fn service_menu(world: &mut IntegrationWorld) -> Result<&ServiceMenu> {
    run(world)
        .services
        .menu
        .as_ref()
        .context("the Services submenu was not opened")
}

#[given(expr = "the runtime starts its services and {int} of {int} are ready")]
fn services_starting(world: &mut IntegrationWorld, ready: usize, total: usize) {
    run(world)
        .cell
        .observer()
        .state_changed(&RuntimeState::StartingServices { ready, total });
}

#[given(
    "the app started the speech recognition service for dictation and adopted the one for commands"
)]
fn dictation_started_commands_adopted(world: &mut IntegrationWorld) {
    let mut statuses: Vec<ServiceStatus> = DICTATION_COPIES
        .iter()
        .enumerate()
        .map(|(i, &port)| {
            let label = format!("dictation: copy {} of {}", i + 1, DICTATION_COPIES.len());
            status(
                ServiceKind::Dictation,
                label,
                port,
                ServiceState::Ready,
                true,
            )
        })
        .collect();
    statuses.push(status(
        ServiceKind::Commands,
        "commands".into(),
        COMMANDS,
        ServiceState::Adopted,
        false,
    ));
    with_services(world, statuses);
}

#[given("the app started the command spotting service and it is Failed")]
fn command_spotting_failed(world: &mut IntegrationWorld) {
    let failed = ServiceState::Failed("exited 5 times".into());
    with_services(
        world,
        vec![status(
            ServiceKind::CommandSpotting,
            "command spotting".into(),
            COMMAND_SPOTTING,
            failed,
            true,
        )],
    );
}

#[when("the user opens the Services submenu")]
fn user_opens_services(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let statuses = run
        .actions
        .as_ref()
        .and_then(voicepuncher::adapters::primary::menubar::macos::actions::MenuActions::services)
        .context("the menu offers no Services submenu")?;
    run.services.menu = Some(
        statuses
            .iter()
            .map(|status| (status.port, service_line(status), offers_restart(status)))
            .collect(),
    );
    Ok(())
}

#[when("the user chooses Restart for it")]
fn user_restarts(world: &mut IntegrationWorld) -> Result<()> {
    let actions = run(world).actions.as_ref().context("no menu actions")?;
    actions.restart_service(COMMAND_SPOTTING);
    Ok(())
}

#[then("it lists the dictation copies as Ready and the speech recognition service for commands as Adopted")]
fn lists_states(world: &mut IntegrationWorld) -> Result<()> {
    let menu = service_menu(world)?;
    let ready = menu
        .iter()
        .filter(|(port, line, _)| {
            DICTATION_COPIES.contains(port)
                && line.starts_with("Dictation")
                && line.ends_with("— Ready")
        })
        .count();
    let adopted = menu
        .iter()
        .any(|(port, line, _)| *port == COMMANDS && line == "Commands — Adopted");
    if ready != DICTATION_COPIES.len() || !adopted {
        bail!("the Services submenu lists {menu:?}");
    }
    Ok(())
}

#[then("Restart is offered for the dictation copies only")]
fn restart_for_dictation_only(world: &mut IntegrationWorld) -> Result<()> {
    let menu = service_menu(world)?;
    let offered: Vec<u16> = menu
        .iter()
        .filter(|(_, _, restart)| *restart)
        .map(|(port, _, _)| *port)
        .collect();
    if offered != DICTATION_COPIES {
        bail!("Restart is offered for {offered:?}");
    }
    Ok(())
}

#[then("the app starts it again")]
fn started_again(world: &mut IntegrationWorld) -> Result<()> {
    let view = run(world).services.view.as_ref().context("no services")?;
    let restarted = view.restarted.lock().unwrap().clone();
    if restarted != [COMMAND_SPOTTING] {
        bail!("the menu asked to restart {restarted:?}");
    }
    Ok(())
}
