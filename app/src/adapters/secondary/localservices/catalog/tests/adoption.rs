//! Services the scripts already run are taken as they run.

use super::*;

#[test]
fn should_take_a_speech_recognition_service_as_the_scripts_run_it() {
    // Given: the scripts run one copy on the service's own port
    let home = Home::complete();
    // When
    let specs = needed(
        &keyword_spotting(),
        ALL,
        &home.machine(),
        &Answering(vec![8081]),
    );
    // Then
    let dictation: Vec<u16> = specs
        .iter()
        .filter(|spec| spec.kind == ServiceKind::Dictation)
        .map(|spec| spec.port)
        .collect();
    assert_eq!(dictation, vec![8081]);
}

#[test]
fn should_take_a_command_spotting_service_as_the_scripts_run_it() {
    // Given: the scripts run the command spotting service on its own port
    let home = Home::complete();
    let scripts_run = Answering(vec![8082]);
    // When
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &scripts_run);
    // Then: one spec on 8082, no copy ports
    let spotting: Vec<u16> = specs
        .iter()
        .filter(|found| found.kind == ServiceKind::CommandSpotting)
        .map(|found| found.port)
        .collect();
    assert_eq!(spotting, [8082]);
}
