use voicepuncher::adapters::secondary::permissions::macos::MacOsPermissionProbe;
use voicepuncher::core::application::permission_probe::PermissionProbe;
use voicepuncher::core::domain::{Permission, PermissionStatus};

const PRIVACY_PAGES: [(Permission, &str); 3] = [
    (
        Permission::Microphone,
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
    ),
    (
        Permission::Accessibility,
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
    ),
    (
        Permission::InputMonitoring,
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
    ),
];

#[test]
fn should_tell_one_of_three_statuses_for_every_permission() {
    // Given
    let probe = MacOsPermissionProbe::new();
    for permission in Permission::ALL {
        // When
        let status = probe.status(permission);
        // Then
        assert!(matches!(
            status,
            PermissionStatus::Granted | PermissionStatus::Denied | PermissionStatus::NotDetermined
        ));
    }
}

#[test]
fn should_link_every_permission_to_its_privacy_page() {
    // Given
    let probe = MacOsPermissionProbe::new();
    for (permission, page) in PRIVACY_PAGES {
        // When
        let url = probe.settings_url(permission);
        // Then
        assert_eq!(url, page);
    }
}

#[test]
fn should_name_the_application_a_bare_test_binary_runs_from() {
    // Given — a test binary has no bundle identifier
    let probe = MacOsPermissionProbe::new();
    // When
    let app = probe.responsible_app();
    // Then
    let name = app.expect("a bare binary names the application it runs from");
    let ancestors = ancestor_commands();
    let nearest_app = ancestors.iter().find_map(|command| {
        let (bundle, _) = command.split_once(".app/Contents/MacOS/")?;
        bundle.rsplit('/').next().map(String::from)
    });
    let parent = ancestors[0]
        .rsplit('/')
        .next()
        .unwrap()
        .trim_start_matches('-');
    assert_eq!(name, nearest_app.unwrap_or_else(|| parent.to_string()));
}

/// The executable paths of this process's ancestors, the parent first, read with `ps`.
fn ancestor_commands() -> Vec<String> {
    let mut commands = Vec::new();
    let mut pid = std::os::unix::process::parent_id();
    while pid > 1 && commands.len() < 16 {
        let output = std::process::Command::new("ps")
            .args(["-o", "ppid=,comm=", "-p", &pid.to_string()])
            .output()
            .expect("ps runs");
        let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let Some((parent_pid, command)) = line.split_once(char::is_whitespace) else {
            break;
        };
        commands.push(command.trim().to_string());
        pid = parent_pid.trim().parse().expect("ps prints a pid");
    }
    commands
}
