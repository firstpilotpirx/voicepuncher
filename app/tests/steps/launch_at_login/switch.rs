//! Steps of the start at login switch in the settings window.

use super::{
    ensure, given, login, refusal_text, session_parts, status, then, when, IntegrationWorld,
    LoginItemStatus, PathBuf, Result, SettingsSession, APPROVAL_NOTE, REFUSAL, SETTINGS,
    UNAVAILABLE_NOTE,
};

#[given("the app is registered as a login item")]
pub(super) fn registered(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::Enabled);
}

#[given("the app is not registered as a login item")]
pub(super) fn not_registered(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::NotRegistered);
}

#[given("the app's login item waits for the user's approval")]
pub(super) fn waits_for_approval(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::RequiresApproval);
}

#[given("the app runs from its build folder")]
pub(super) fn build_folder(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::Unavailable);
}

#[given("the system refuses to register it")]
pub(super) fn refuses(world: &mut IntegrationWorld) {
    *login(world).item.refuses.lock().unwrap() = true;
}

#[given("the user removed the app from the login items in System Settings")]
pub(super) fn removed_in_system_settings(world: &mut IntegrationWorld) {
    login(world).item.set(LoginItemStatus::NotRegistered);
}

#[given("the user opens the General tab")]
#[when("the user opens the General tab")]
#[when("the settings window comes to the front again")]
pub(super) fn open_general(world: &mut IntegrationWorld) {
    let run = login(world);
    run.view = Some(run.switch().view());
}

#[when("the user turns the login switch on")]
pub(super) fn turn_on(world: &mut IntegrationWorld) {
    turn(world, true);
}

#[when("the user turns the login switch off")]
pub(super) fn turn_off(world: &mut IntegrationWorld) {
    turn(world, false);
}

pub(super) fn turn(world: &mut IntegrationWorld, on: bool) {
    let run = login(world);
    let turned = run.switch().turn(on);
    run.status_line = turned.refusal.as_deref().map(refusal_text);
    run.view = Some(turned.view);
}

#[then("the login switch is on")]
pub(super) fn switch_on(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(login(world).view()?.on, "the switch is off");
    Ok(())
}

#[then("the login switch is off")]
pub(super) fn switch_off(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(!login(world).view()?.on, "the switch is on");
    Ok(())
}

#[then("no approval note is shown")]
pub(super) fn no_note(world: &mut IntegrationWorld) -> Result<()> {
    let view = login(world).view()?;
    ensure!(view.note.is_none() && !view.approval_button, "{view:?}");
    Ok(())
}

#[then("a note says it waits for approval in System Settings")]
pub(super) fn approval_note(world: &mut IntegrationWorld) -> Result<()> {
    let note = login(world).view()?.note;
    ensure!(note == Some(APPROVAL_NOTE), "{note:?}");
    Ok(())
}

#[then("a button opens the Login Items settings")]
pub(super) fn approval_button(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(login(world).view()?.approval_button, "no button");
    Ok(())
}

#[then(regex = r"^the login switch is (enabled|disabled)$")]
pub(super) fn enabled(world: &mut IntegrationWorld, state: String) -> Result<()> {
    let enabled = login(world).view()?.enabled;
    ensure!(enabled == (state == "enabled"), "the switch is not {state}");
    Ok(())
}

#[then("a note says it is available when voicepuncher runs as an installed app")]
pub(super) fn unavailable_note(world: &mut IntegrationWorld) -> Result<()> {
    let note = login(world).view()?.note;
    ensure!(note == Some(UNAVAILABLE_NOTE), "{note:?}");
    Ok(())
}

#[then("nothing was registered or unregistered")]
pub(super) fn nothing_changed(world: &mut IntegrationWorld) -> Result<()> {
    let changes = *login(world).item.changes.lock().unwrap();
    ensure!(changes == 0, "{changes} changes asked of the system");
    Ok(())
}

#[then("the app is registered as a login item")]
pub(super) fn is_registered(world: &mut IntegrationWorld) -> Result<()> {
    let status = status(world);
    ensure!(status == LoginItemStatus::Enabled, "{status:?}");
    Ok(())
}

#[then("the app is not registered as a login item")]
pub(super) fn is_not_registered(world: &mut IntegrationWorld) -> Result<()> {
    let status = status(world);
    ensure!(status == LoginItemStatus::NotRegistered, "{status:?}");
    Ok(())
}

#[then("the configuration file is left as it was")]
pub(super) fn file_unchanged(world: &mut IntegrationWorld) -> Result<()> {
    let file = std::fs::read_to_string(login(world).path())?;
    ensure!(file == SETTINGS, "the file became {file:?}");
    Ok(())
}

#[then("closing the settings window asks nothing")]
pub(super) fn closing_asks_nothing(world: &mut IntegrationWorld) -> Result<()> {
    let path = login(world).path();
    let session = SettingsSession::open(&path, &PathBuf::from("/"), session_parts())?;
    ensure!(!session.is_dirty(), "the settings have unsaved edits");
    Ok(())
}

#[then("the status line shows the system's reason")]
pub(super) fn reason_shown(world: &mut IntegrationWorld) -> Result<()> {
    let line = login(world).status_line.clone().unwrap_or_default();
    ensure!(line.contains(REFUSAL), "the status line says {line:?}");
    Ok(())
}
