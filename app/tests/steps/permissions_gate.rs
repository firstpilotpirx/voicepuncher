//! Drives the real runtime of the app-runtime steps with permissions the scenario sets, as
//! System Settings would: the runtime state Waiting for permissions is what opens the
//! Permissions window, and the gate's report is what its rows show.

use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use cucumber::{given, then, when};

use voicepuncher::core::domain::{Permission, PermissionStatus};

use crate::steps::app_runtime::run;
use crate::IntegrationWorld;

/// Longest the log may take to show a change.
const REACTION: Duration = Duration::from_secs(2);

fn named(name: &str) -> Permission {
    Permission::ALL
        .into_iter()
        .find(|p| p.name() == name)
        .unwrap_or_else(|| panic!("no permission is named {name}"))
}

fn row_shows(world: &mut IntegrationWorld, name: &str, words: &str) -> Result<()> {
    let status = run(world).permissions().gate().report().status(named(name));
    if status.words() != words {
        bail!("the {name} row shows {}, not {words}", status.words());
    }
    Ok(())
}

#[given("Microphone, Accessibility and Input Monitoring are granted")]
pub(crate) fn all_granted(world: &mut IntegrationWorld) {
    for permission in Permission::ALL {
        run(world)
            .permissions()
            .set(permission, PermissionStatus::Granted);
    }
}

#[given("no permission is granted")]
fn none_granted(world: &mut IntegrationWorld) {
    for permission in Permission::ALL {
        run(world)
            .permissions()
            .set(permission, PermissionStatus::Denied);
    }
}

#[given("Accessibility is not granted and the other two are")]
fn accessibility_missing(world: &mut IntegrationWorld) {
    all_granted(world);
    run(world)
        .permissions()
        .set(Permission::Accessibility, PermissionStatus::Denied);
}

#[given("the app runs without a pipeline because Input Monitoring is missing")]
fn waits_for_input_monitoring(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.permissions()
        .set(Permission::InputMonitoring, PermissionStatus::Denied);
    run.start_waiting()
}

#[given("the app never asked for Microphone")]
fn microphone_never_asked(world: &mut IntegrationWorld) {
    run(world)
        .permissions()
        .set(Permission::Microphone, PermissionStatus::NotDetermined);
}

#[given("the user denied Microphone")]
fn microphone_denied(world: &mut IntegrationWorld) {
    run(world)
        .permissions()
        .set(Permission::Microphone, PermissionStatus::Denied);
}

#[given("the pipeline runs and no dictation is in progress")]
fn pipeline_idle(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.launch();
    run.assembled(1)
}

#[given("the pipeline stopped because Accessibility was revoked")]
fn stopped_for_accessibility(world: &mut IntegrationWorld) -> Result<()> {
    pipeline_idle(world)?;
    let run = run(world);
    run.permissions()
        .set(Permission::Accessibility, PermissionStatus::Denied);
    run.waits_for(&[Permission::Accessibility], &[Permission::Accessibility])
}

#[given(
    "the app runs with Microphone granted, Accessibility not granted and Input Monitoring not asked yet"
)]
fn mixed_statuses(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.permissions()
        .set(Permission::Accessibility, PermissionStatus::Denied);
    run.permissions()
        .set(Permission::InputMonitoring, PermissionStatus::NotDetermined);
    run.start_waiting()
}

#[when("the app launches in hotkey mode")]
#[when("the Permissions window shows")]
fn launches(world: &mut IntegrationWorld) {
    run(world).launch();
}

#[when(regex = r"^the user grants (Microphone|Accessibility|Input Monitoring)(?: again)?$")]
fn grants(world: &mut IntegrationWorld, name: String) {
    run(world)
        .permissions()
        .set(named(&name), PermissionStatus::Granted);
}

#[when(
    regex = r"^the user removes (Microphone|Accessibility|Input Monitoring) in System Settings$"
)]
fn removes(world: &mut IntegrationWorld, name: String) {
    run(world)
        .permissions()
        .set(named(&name), PermissionStatus::Denied);
}

#[when("the user transcribes a recording file")]
fn transcribes_file(world: &mut IntegrationWorld) -> Result<()> {
    run(world).transcribe_file()
}

#[then("the dictation pipeline starts")]
fn pipeline_starts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.started_ok()?;
    run.assembled(1)
}

#[then("the Permissions window does not open")]
fn window_stays_closed(world: &mut IntegrationWorld) -> Result<()> {
    if run(world).ever_waited_for_permissions() {
        bail!("the runtime waited for permissions");
    }
    Ok(())
}

#[then("the Permissions window opens with the Accessibility row marked not granted")]
fn window_marks_accessibility(world: &mut IntegrationWorld) -> Result<()> {
    run(world).waits_for(&[Permission::Accessibility], &[])?;
    row_shows(world, "Accessibility", "not granted")
}

#[then("the process keeps running")]
fn process_keeps_running(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.started_ok()?;
    if !run.same_process() {
        bail!("the process changed");
    }
    Ok(())
}

#[then("within 2 s the dictation pipeline starts in the same process")]
fn starts_in_same_process(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.assembled(1)?;
    if !run.same_process() {
        bail!("the process changed");
    }
    Ok(())
}

#[then(regex = r"^the Microphone row shows (Not asked yet|Not granted)$")]
fn microphone_row(world: &mut IntegrationWorld, shown: String) -> Result<()> {
    run(world).waits_for(&[Permission::Microphone], &[])?;
    row_shows(world, "Microphone", &shown.to_lowercase())
}

#[then("the microphone is not opened to find out")]
fn microphone_not_opened(world: &mut IntegrationWorld) -> Result<()> {
    run(world).no_pipeline_runs()
}

#[then("within 5 s the pipeline stops")]
fn pipeline_stops(world: &mut IntegrationWorld) -> Result<()> {
    run(world).pipeline_dropped(1)
}

#[then(
    "the Permissions window opens with the Accessibility row marked not granted and the line saying Accessibility was revoked"
)]
fn window_marks_revoked(world: &mut IntegrationWorld) -> Result<()> {
    run(world).waits_for(&[Permission::Accessibility], &[Permission::Accessibility])?;
    row_shows(world, "Accessibility", "not granted")
}

#[then("within 5 s the recording is cancelled with the cancel sound and nothing is pasted")]
fn recording_cancelled(world: &mut IntegrationWorld) -> Result<()> {
    run(world).cancelled_and_nothing_pasted()
}

#[then("a failure notice says the Microphone permission was revoked")]
fn notice_names_microphone(world: &mut IntegrationWorld) -> Result<()> {
    let message = run(world).notice(1)?;
    if !message.contains("Microphone permission was revoked") {
        bail!("the notice says {message:?}");
    }
    Ok(())
}

#[then("the dictation pipeline starts again without a relaunch")]
fn starts_again(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.assembled(2)?;
    if !run.same_process() {
        bail!("the process changed");
    }
    Ok(())
}

#[then(
    "the log has one line naming Microphone granted, Accessibility granted and Input Monitoring not asked yet"
)]
fn change_logged(_world: &mut IntegrationWorld) -> Result<()> {
    let wanted =
        "permissions: microphone granted, accessibility granted, input monitoring not asked yet";
    let log = crate::steps::voice_control::pipeline_log();
    let deadline = Instant::now() + REACTION;
    while !log.permissions.lock().unwrap().iter().any(|l| l == wanted) {
        if Instant::now() >= deadline {
            bail!(
                "not in the log: {wanted}; lines: {:?}",
                log.permissions.lock().unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

#[then("the transcript is printed")]
fn transcript_printed(world: &mut IntegrationWorld) -> Result<()> {
    run(world).printed()
}

#[then("no permission is checked")]
fn no_permission_checked(world: &mut IntegrationWorld) -> Result<()> {
    let checks = run(world).permissions().checks();
    if checks != 0 {
        bail!("the permissions were checked {checks} times");
    }
    Ok(())
}
