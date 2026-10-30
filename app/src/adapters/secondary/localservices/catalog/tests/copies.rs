//! The services and copies a configuration needs and how each is started.

use super::*;

#[test]
pub(super) fn should_need_only_the_dictation_copies_when_voice_control_is_off() {
    // Given
    let home = Home::complete();
    let needs = Needs { voice_control: false };
    // When
    let specs = needed(&keyword_spotting(), needs, &home.machine(), &NOTHING_RUNS);
    // Then
    assert_eq!(
        kinds_and_ports(&specs),
        vec![
            (ServiceKind::Dictation, 18082),
            (ServiceKind::Dictation, 18083),
            (ServiceKind::Dictation, 18084),
        ]
    );
    assert!(specs.iter().all(|spec| spec.problem.is_none()), "{specs:?}");
}

#[test]
pub(super) fn should_need_the_command_services_when_voice_control_is_on() {
    // Given
    let home = Home::complete();
    // When
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let commands: Vec<(ServiceKind, u16)> = kinds_and_ports(&specs)
        .into_iter()
        .filter(|(kind, _)| matches!(kind, ServiceKind::Commands | ServiceKind::CommandSpotting))
        .collect();
    assert_eq!(
        commands,
        vec![
            (ServiceKind::Commands, 18086),
            (ServiceKind::Commands, 18087),
            (ServiceKind::Commands, 18088),
            (ServiceKind::CommandSpotting, 49083),
            (ServiceKind::CommandSpotting, 49084),
        ]
    );
}

#[test]
pub(super) fn should_run_one_copy_on_the_service_port_with_every_core() {
    // Given
    let home = Home::complete();
    let mut cfg = keyword_spotting();
    cfg.services.dictation_copies = 1;
    // When
    let specs = needed(&cfg, ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let dictation = &specs[0];
    assert_eq!(dictation.port, 8081);
    assert_eq!(dictation.label, "speech recognition service for dictation");
    assert!(dictation.args.windows(2).any(|pair| pair == ["-t", "12"]));
}

#[test]
pub(super) fn should_split_the_cores_between_the_copies() {
    assert_eq!(threads(12, 3), 4);
    assert_eq!(threads(2, 3), 1);
    assert_eq!(threads(8, 1), 8);
}

#[test]
pub(super) fn should_run_one_configured_command_spotting_copy_on_the_service_port() {
    // Given
    let home = Home::complete();
    let mut cfg = keyword_spotting();
    cfg.services.command_spotting_copies = 1;
    // When
    let specs = needed(&cfg, ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let spotting: Vec<(u16, &str)> = specs
        .iter()
        .filter(|spec| spec.kind == ServiceKind::CommandSpotting)
        .map(|spec| (spec.port, spec.label.as_str()))
        .collect();
    assert_eq!(spotting, vec![(8082, "command spotting service")]);
}

#[test]
pub(super) fn should_label_each_command_spotting_copy() {
    let home = Home::complete();
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &NOTHING_RUNS);
    let labels: Vec<&str> = specs
        .iter()
        .filter(|spec| spec.kind == ServiceKind::CommandSpotting)
        .map(|spec| spec.label.as_str())
        .collect();
    assert_eq!(
        labels,
        vec![
            "command spotting service: copy 1 of 2",
            "command spotting service: copy 2 of 2"
        ]
    );
}

#[test]
pub(super) fn should_start_each_command_spotting_copy_on_its_own_port() {
    // Given
    let home = Home::complete();
    let mut cfg = keyword_spotting();
    cfg.services.command_spotting_copies = 3;
    // When
    let specs = needed(&cfg, ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let spotting: Vec<&ServiceSpec> = specs
        .iter()
        .filter(|spec| spec.kind == ServiceKind::CommandSpotting)
        .collect();
    assert_eq!(spotting.len(), 3);
    for spec in spotting {
        let port = spec.port.to_string();
        assert!(
            spec.args
                .windows(2)
                .any(|pair| pair == ["--port", port.as_str()]),
            "{:?}",
            spec.args
        );
    }
}

#[test]
pub(super) fn should_health_check_every_service_it_starts_each_copy_too() {
    // Given
    let home = Home::complete();
    // When
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let unchecked: Vec<&str> = specs
        .iter()
        .filter(|spec| !spec.health_checked)
        .map(|spec| spec.label.as_str())
        .collect();
    assert!(unchecked.is_empty(), "not health-checked: {unchecked:?}");
}

#[test]
pub(super) fn should_log_each_service_to_a_file_named_by_its_program_and_port() {
    let home = Home::complete();
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &NOTHING_RUNS);
    let logs: Vec<PathBuf> = specs.iter().map(|spec| spec.log_file.clone()).collect();
    let in_logs = home.path().join(".voicepuncher/logs");
    assert!(logs.contains(&in_logs.join("whisper-server-18082.log")));
    assert!(logs.contains(&in_logs.join("voicepuncher-command-spotting-49083.log")));
}
