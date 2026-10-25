//! The menu items for models, permissions and services.

use super::*;

#[test]
pub(super) fn should_show_the_waiting_icon_while_waiting_for_models() {
    assert_eq!(
        state(None, Some(waiting_for_models())).icon(),
        IconState::Waiting
    );
}

#[test]
pub(super) fn should_point_at_the_download_window_while_waiting_for_models() {
    assert_eq!(
        state(None, Some(waiting_for_models())).status_line(),
        "Waiting for models — open Download models…"
    );
}

#[test]
pub(super) fn should_offer_download_models_right_under_permissions() {
    let permissions = MENU
        .iter()
        .position(|entry| *entry == MenuEntry::Permissions)
        .expect("the menu has Permissions…");
    assert_eq!(MENU.get(permissions + 1), Some(&MenuEntry::DownloadModels));
    assert_eq!(MenuEntry::DownloadModels.title(), Some("Download models…"));
}

pub(super) fn waiting_for_models() -> RuntimeState {
    RuntimeState::WaitingForModels {
        folder: "/Users/me/Library/Application Support/voicepuncher/models".into(),
        missing: vec!["ggml-large-v3-turbo-q5_0.bin"],
    }
}

#[test]
pub(super) fn should_offer_permissions_right_under_the_status_line() {
    let status = MENU
        .iter()
        .position(|entry| *entry == MenuEntry::StatusLine)
        .expect("the menu has a status line");
    assert_eq!(MENU.get(status + 1), Some(&MenuEntry::Permissions));
    assert_eq!(MenuEntry::Permissions.title(), Some("Permissions…"));
}

#[test]
pub(super) fn should_count_the_services_ready_while_they_start() {
    // Given
    let starting = state(
        None,
        Some(RuntimeState::StartingServices { ready: 2, total: 5 }),
    );
    // When / Then
    assert_eq!(starting.icon(), IconState::Waiting);
    assert_eq!(starting.status_line(), "Starting services… (2 of 5 ready)");
}

#[test]
pub(super) fn should_show_a_service_that_cannot_start_as_failed() {
    let failed = state(
        None,
        Some(RuntimeState::Failed(
            "command spotting service: model missing".into(),
        )),
    );
    assert_eq!(failed.icon(), IconState::Failed);
    assert_eq!(
        failed.status_line(),
        "Failed: command spotting service: model missing"
    );
}

pub(super) fn service(
    label: &str,
    state: crate::core::domain::ServiceState,
    owned: bool,
) -> ServiceStatus {
    ServiceStatus {
        kind: crate::core::domain::ServiceKind::Dictation,
        label: label.into(),
        port: 18082,
        state,
        owned,
        log_file: std::path::PathBuf::from("/logs/whisper-server-18082.log"),
    }
}

#[test]
pub(super) fn should_list_a_service_with_its_state_in_words_and_restart_only_owned_ones() {
    use crate::core::domain::ServiceState;
    // Given
    let copy = service(
        "speech recognition service for dictation: copy 1 of 3",
        ServiceState::Ready,
        true,
    );
    let adopted = service("command spotting service", ServiceState::Adopted, false);
    // When / Then
    assert_eq!(
        service_line(&copy),
        "Speech recognition service for dictation: copy 1 of 3 — Ready"
    );
    assert_eq!(service_line(&adopted), "Command spotting service — Adopted");
    assert!(offers_restart(&copy));
    assert!(!offers_restart(&adopted));
}
