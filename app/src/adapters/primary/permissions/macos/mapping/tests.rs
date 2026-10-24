use super::*;

fn report(statuses: [PermissionStatus; 3], responsible_app: Option<&str>) -> PermissionReport {
    PermissionReport {
        statuses: Permission::ALL.into_iter().zip(statuses).collect(),
        responsible_app: responsible_app.map(str::to_owned),
    }
}

use PermissionStatus::{Denied, Granted, NotDetermined};

#[test]
fn should_show_each_status_as_its_badge() {
    assert_eq!(badge(Shown::Status(Granted)), ("Granted", BadgeMark::Green));
    assert_eq!(
        badge(Shown::Status(Denied)),
        ("Not granted", BadgeMark::Red)
    );
    assert_eq!(
        badge(Shown::Status(NotDetermined)),
        ("Not asked yet", BadgeMark::Grey)
    );
    assert_eq!(badge(Shown::Checking), ("Checking…", BadgeMark::Spinner));
}

#[test]
fn should_offer_request_for_not_asked_yet_settings_for_denied_and_nothing_for_granted() {
    assert_eq!(
        button(Shown::Status(NotDetermined)),
        Some(RowButton::Request)
    );
    assert_eq!(
        button(Shown::Status(Denied)),
        Some(RowButton::OpenSystemSettings)
    );
    assert_eq!(button(Shown::Status(Granted)), None);
    assert_eq!(button(Shown::Checking), None);
    assert_eq!(RowButton::Request.title(), "Request…");
    assert_eq!(
        RowButton::OpenSystemSettings.title(),
        "Open System Settings"
    );
}

#[test]
fn should_name_each_permission_with_why_and_symbol() {
    assert_eq!(why(Permission::Microphone), "to hear your dictation");
    assert_eq!(
        why(Permission::Accessibility),
        "to paste the text into the app you are typing in"
    );
    assert_eq!(
        why(Permission::InputMonitoring),
        "to notice the hotkey in any app"
    );
    assert_eq!(symbol(Permission::Microphone), "mic");
    assert_eq!(symbol(Permission::Accessibility), "accessibility");
    assert_eq!(symbol(Permission::InputMonitoring), "keyboard");
}

#[test]
fn should_show_one_row_per_permission_in_order() {
    let model = WindowModel::new(&report([Granted, Denied, NotDetermined], None), &[], &[]);
    let rows: Vec<_> = model.rows.iter().map(|r| (r.permission, r.shown)).collect();
    assert_eq!(
        rows,
        vec![
            (Permission::Microphone, Shown::Status(Granted)),
            (Permission::Accessibility, Shown::Status(Denied)),
            (Permission::InputMonitoring, Shown::Status(NotDetermined)),
        ]
    );
}

#[test]
fn should_show_checking_while_a_request_is_not_answered() {
    let pending = [Permission::InputMonitoring];
    let asking = WindowModel::new(
        &report([Granted, Granted, NotDetermined], None),
        &pending,
        &[],
    );
    assert_eq!(asking.rows[2].shown, Shown::Checking);
    let answered = WindowModel::new(&report([Granted, Granted, Denied], None), &pending, &[]);
    assert_eq!(answered.rows[2].shown, Shown::Status(Denied));
}

#[test]
fn should_name_the_responsible_app_when_run_from_another_app() {
    let model = WindowModel::new(&report([Granted; 3], Some("iTerm2")), &[], &[]);
    assert_eq!(
        model.developer_line.as_deref(),
        Some("voicepuncher runs from iTerm2 — grant the permissions to iTerm2.")
    );
}

#[test]
fn should_name_no_other_app_when_run_from_the_bundle() {
    let model = WindowModel::new(&report([Granted; 3], None), &[], &[]);
    assert_eq!(model.developer_line, None);
}

#[test]
fn should_say_which_permission_was_revoked() {
    let model = WindowModel::new(
        &report([Granted, Denied, Granted], None),
        &[],
        &[Permission::Accessibility],
    );
    assert_eq!(
        model.revoked_line.as_deref(),
        Some("Accessibility was revoked — dictation stopped")
    );
    let none = WindowModel::new(&report([Granted, Denied, Granted], None), &[], &[]);
    assert_eq!(none.revoked_line, None);
}

#[test]
fn should_enable_continue_only_when_every_permission_is_granted() {
    let missing = WindowModel::new(&report([Granted, Denied, Granted], None), &[], &[]);
    assert!(!missing.continue_enabled);
    let all = WindowModel::new(&report([Granted; 3], None), &[], &[]);
    assert!(all.continue_enabled);
}

#[test]
fn should_name_the_accessibility_pane_by_the_macos_version() {
    assert_eq!(
        pane_name(Permission::Accessibility, 27),
        "Device Control and Data Access"
    );
    assert_eq!(pane_name(Permission::Accessibility, 26), "Accessibility");
    assert_eq!(pane_name(Permission::Microphone, 27), "Microphone");
    assert_eq!(
        pane_name(Permission::InputMonitoring, 27),
        "Input Monitoring"
    );
    let line = row_line(Permission::Accessibility, Shown::Status(Denied), false, 27);
    assert_eq!(
        line,
        "Switch voicepuncher on in Device Control and Data Access"
    );
    let before = row_line(Permission::Accessibility, Shown::Status(Denied), false, 26);
    assert_eq!(before, "Switch voicepuncher on in Accessibility");
}

#[test]
fn should_hint_to_add_the_app_again_only_after_system_settings_opened() {
    let opened = row_line(Permission::Accessibility, Shown::Status(Denied), true, 27);
    assert_eq!(
        opened,
        "Already on? Remove voicepuncher with − and add it again with +."
    );
    let granted = row_line(Permission::Accessibility, Shown::Status(Granted), true, 27);
    assert_eq!(granted, why(Permission::Accessibility));
    let not_opened = row_line(Permission::Accessibility, Shown::Status(Denied), false, 27);
    assert!(!not_opened.contains('+'), "{not_opened}");
}

#[test]
fn should_hint_to_add_input_monitoring_with_plus() {
    let line = row_line(Permission::InputMonitoring, Shown::Status(Denied), true, 27);
    assert_eq!(line, "Not listed? Add voicepuncher with +.");
}

#[test]
fn should_request_open_and_reveal_for_input_monitoring() {
    use SettingsStep::{OpenPane, Request, RevealBundle};
    assert_eq!(
        settings_steps(Permission::InputMonitoring, true),
        [Request, OpenPane, RevealBundle]
    );
    assert_eq!(
        settings_steps(Permission::InputMonitoring, false),
        [Request, OpenPane]
    );
    assert_eq!(settings_steps(Permission::Accessibility, true), [OpenPane]);
    assert_eq!(settings_steps(Permission::Microphone, true), [OpenPane]);
}

#[test]
fn should_refresh_every_second() {
    assert_eq!(REFRESH_MS, 1_000);
}
