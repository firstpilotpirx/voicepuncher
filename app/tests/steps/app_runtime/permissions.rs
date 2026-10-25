//! Steps of the runtime waiting for a permission.

use super::{
    bail, given, run, then, when, Event, IntegrationWorld, Permission, PermissionStatus, Result,
    RuntimeState, FIRST_HOTKEY,
};

#[given("an observer registered before the runtime starts")]
pub(super) fn observer_registered(world: &mut IntegrationWorld) {
    // every run registers its recording observer before it starts the runtime
    run(world);
}

#[given("Input Monitoring is not granted")]
pub(super) fn input_monitoring_not_granted(world: &mut IntegrationWorld) {
    run(world)
        .permissions
        .set(Permission::InputMonitoring, PermissionStatus::Denied);
}

#[then("the observer is told Waiting for permissions naming Input Monitoring")]
pub(super) fn told_waiting_input_monitoring(world: &mut IntegrationWorld) -> Result<()> {
    run(world).waits_for(&[Permission::InputMonitoring], &[])
}

#[given("the runtime is Waiting for permissions")]
pub(super) fn runtime_waiting(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.permissions
        .set(Permission::Accessibility, PermissionStatus::Denied);
    run.start_waiting()
}

#[when("the last missing permission is granted")]
pub(super) fn last_missing_granted(world: &mut IntegrationWorld) {
    run(world)
        .permissions
        .set(Permission::Accessibility, PermissionStatus::Granted);
}

#[then("the observer is told Starting and then Ready")]
pub(super) fn told_starting_then_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    let waiting =
        run.position(|e| matches!(e, Event::State(RuntimeState::WaitingForPermissions { .. })))?;
    let states: Vec<_> = run.events()[waiting + 1..]
        .iter()
        .filter_map(|e| match e {
            Event::State(state) => Some(state.clone()),
            _ => None,
        })
        .collect();
    if !matches!(
        states.as_slice(),
        [RuntimeState::Starting, RuntimeState::Ready { .. }]
    ) {
        bail!("expected Starting and then Ready, got {states:?}");
    }
    Ok(())
}

#[then("the dictation pipeline runs")]
pub(super) fn pipeline_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.assembled(1)?;
    if run.recordings_after(FIRST_HOTKEY) != 1 {
        bail!("the hotkey started no recording: {:?}", run.events());
    }
    Ok(())
}

#[when("Accessibility is revoked")]
pub(super) fn accessibility_revoked(world: &mut IntegrationWorld) {
    run(world)
        .permissions
        .set(Permission::Accessibility, PermissionStatus::Denied);
}

#[then("the observer is told Waiting for permissions naming Accessibility")]
pub(super) fn told_waiting_accessibility(world: &mut IntegrationWorld) -> Result<()> {
    run(world).waits_for(&[Permission::Accessibility], &[Permission::Accessibility])
}
