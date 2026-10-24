use super::*;
use crate::core::domain::Permission;

fn waiting_for(missing: &[Permission]) -> StatusItemState {
    let runtime =
        RuntimeState::WaitingForPermissions { missing: missing.to_vec(), revoked: vec![] };
    StatusItemState { dictation: None, runtime: Some(runtime), commands_unavailable: false }
}

#[test]
fn should_name_the_missing_permissions_with_their_own_icon() {
    // Given
    let one = waiting_for(&[Permission::Accessibility]);
    let two = waiting_for(&[Permission::Accessibility, Permission::InputMonitoring]);
    // Then
    assert_eq!(one.icon(), IconState::MissingPermission);
    assert_ne!(
        IconState::MissingPermission.picture(),
        IconState::Waiting.picture()
    );
    assert_eq!(one.status_line(), "Waiting for Accessibility…");
    assert_eq!(
        two.status_line(),
        "Waiting for Accessibility and Input Monitoring…"
    );
}

#[test]
fn should_let_the_status_line_open_permissions_only_while_waiting_for_them() {
    // Given
    let waiting = waiting_for(&[Permission::Accessibility]);
    let mut recording = waiting_for(&[Permission::Accessibility]);
    recording.apply(OverlaySignal::Recording);
    let opens = waiting.status_opens_permissions();
    let starting = StatusItemState { runtime: Some(RuntimeState::Starting), ..waiting };
    // Then
    assert!(opens);
    assert!(!recording.status_opens_permissions());
    assert!(!starting.status_opens_permissions());
}
