//! Steps of the first launch's question and of a start at login.

use super::{
    ensure, given, launch_line, login, status, then, when, Config, ConfigApplier, InputDevices,
    IntegrationWorld, LoginItemStatus, OfferAnswer, Result, SessionParts, StatusListener,
    ASKED_BEFORE, LAUNCHED_AS_LOGIN_ITEM, LAUNCHED_AT_LOGIN, OPEN_APPLICATION,
};

#[given("the installed app has never asked about starting at login")]
pub(super) fn never_asked(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::NotRegistered);
}

#[given("the configuration file records that the question was asked")]
pub(super) fn asked_before(world: &mut IntegrationWorld) {
    let run = login(world);
    run.item.set(LoginItemStatus::NotRegistered);
    std::fs::write(run.path(), ASKED_BEFORE).expect("the configuration file is written");
}

#[given("the app reached Ready and asked whether to start at login")]
#[when("the app reaches Ready")]
#[when("a saved setting restarts the pipeline and the app reaches Ready again")]
pub(super) fn reaches_ready(world: &mut IntegrationWorld) {
    login(world).reach_ready();
}

#[when("the app reaches Ready after a relaunch")]
pub(super) fn reaches_ready_after_relaunch(world: &mut IntegrationWorld) {
    let run = login(world);
    run.offer = None;
    run.reach_ready();
}

#[when("the user answers Not Now")]
pub(super) fn answers_not_now(world: &mut IntegrationWorld) -> Result<()> {
    login(world).answer(OfferAnswer::NotNow)
}

#[then("the app asks whether to start at login")]
pub(super) fn asks(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(login(world).asked == Some(true), "the app did not ask");
    Ok(())
}

#[then("answering Start at Login registers the app as a login item")]
pub(super) fn start_at_login_registers(world: &mut IntegrationWorld) -> Result<()> {
    login(world).answer(OfferAnswer::StartAtLogin)?;
    let status = status(world);
    ensure!(status == LoginItemStatus::Enabled, "{status:?}");
    Ok(())
}

#[then("the configuration file records that the question was asked")]
pub(super) fn records_asked(world: &mut IntegrationWorld) -> Result<()> {
    let saved = voicepuncher::config::load(&login(world).path())?;
    let asked = saved.app.launch_at_login_asked;
    ensure!(asked, "the file does not record the question");
    Ok(())
}

#[then("the app asks nothing")]
#[then("the app asks nothing more")]
pub(super) fn asks_nothing(world: &mut IntegrationWorld) -> Result<()> {
    let asked = login(world).asked;
    ensure!(asked == Some(false), "asked: {asked:?}");
    Ok(())
}

pub(super) struct NoApplier;

impl ConfigApplier for NoApplier {
    fn apply(&self, _config: Config, _listener: StatusListener) {}
}

pub(super) struct NoDevices;

impl InputDevices for NoDevices {
    fn names(&self) -> Vec<String> {
        Vec::new()
    }
}

pub(super) fn session_parts() -> SessionParts {
    SessionParts {
        overlay: Box::new(|_| {}),
        applier: Box::new(NoApplier),
        devices: Box::new(NoDevices),
    }
}

#[when("the system starts the app at login")]
pub(super) fn started_at_login(world: &mut IntegrationWorld) {
    login(world).launch_line = launch_line(Some(OPEN_APPLICATION), Some(LAUNCHED_AS_LOGIN_ITEM));
}

#[then("the log says the app was launched at login")]
pub(super) fn logged_at_login(world: &mut IntegrationWorld) -> Result<()> {
    let line = login(world).launch_line;
    ensure!(
        line == Some(LAUNCHED_AT_LOGIN),
        "the launch logged {line:?}"
    );
    Ok(())
}
