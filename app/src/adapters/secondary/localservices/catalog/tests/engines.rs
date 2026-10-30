//! Where an engine is found and how its version is named.

use super::*;

#[test]
pub(super) fn should_name_the_command_model_it_looked_for_when_it_is_not_on_disk() {
    // Given
    let home = Home::complete();
    let missing = home.path().join("elsewhere/ggml-commands.bin");
    let mut machine = home.machine();
    machine.env.insert(
        "VOICEPUNCHER_COMMAND_WHISPER_MODEL".into(),
        missing.display().to_string(),
    );
    // When
    let specs = needed(&keyword_spotting(), ALL, &machine, &NOTHING_RUNS);
    // Then
    let commands = specs
        .iter()
        .find(|spec| spec.kind == ServiceKind::Commands)
        .unwrap();
    let problem = commands.problem.as_deref().unwrap();
    assert!(
        problem.contains(&missing.display().to_string()),
        "{problem}"
    );
}

#[test]
pub(super) fn should_name_a_missing_engine_with_how_to_get_it() {
    // Given
    let home = Home::complete();
    fs::remove_file(home.path().join("bin").join(WHISPER)).unwrap();
    // When
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let problem = specs[0].problem.clone().unwrap();
    assert!(problem.contains("whisper-server not found"), "{problem}");
    assert!(problem.contains("VOICEPUNCHER_WHISPER_BINARY"), "{problem}");
}

#[test]
pub(super) fn should_find_an_engine_from_the_environment_then_the_bundle_then_the_path() {
    // Given
    let home = Home::complete();
    let bundled = home.path().join("app/engines/whisper/whisper-server");
    let chosen = home.path().join("elsewhere/whisper-server");
    touch(&bundled);
    touch(&chosen);
    let mut machine = home.machine();
    let lookup_program = |machine: &Machine| {
        needed(&keyword_spotting(), ALL, machine, &NOTHING_RUNS)
            .into_iter()
            .find(|spec| spec.kind == ServiceKind::Dictation)
            .unwrap()
            .program
    };
    // When
    let from_bundle = lookup_program(&machine);
    machine.env.insert(
        "VOICEPUNCHER_WHISPER_BINARY".into(),
        chosen.display().to_string(),
    );
    let from_env = lookup_program(&machine);
    // Then
    assert_eq!(from_bundle, bundled);
    assert_eq!(from_env, chosen);
}

#[test]
pub(super) fn should_take_every_engine_from_its_folder_in_the_bundle_before_the_path() {
    // Given
    let home = Home::complete();
    let engines = home.path().join("app/engines");
    for (folder, program) in [("whisper", WHISPER), ("command-spotting", COMMAND_SPOTTING)] {
        touch(&engines.join(folder).join(program));
    }
    // When
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    assert!(!specs.is_empty());
    for spec in &specs {
        assert!(
            spec.program.starts_with(&engines),
            "{} runs {}",
            spec.label,
            spec.program.display()
        );
    }
}

#[test]
pub(super) fn should_take_every_engine_from_the_path_outside_a_bundle() {
    // Given
    let home = Home::complete();
    // When
    let specs = needed(&keyword_spotting(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    for spec in &specs {
        assert_eq!(
            spec.program.parent(),
            Some(home.path().join("bin").as_path())
        );
    }
}

#[test]
pub(super) fn should_name_a_bundled_engine_with_the_version_of_its_folder() {
    // Given
    let home = Home::complete();
    let program = home.path().join("app/engines/whisper").join(WHISPER);
    touch(&program);
    fs::write(program.with_file_name("VERSION"), "1.9.5\n").unwrap();
    // When
    let origin = engine_origin(&program, &home.machine());
    // Then
    assert_eq!(origin.to_string(), "whisper-server 1.9.5 from the bundle");
}

#[test]
pub(super) fn should_name_a_homebrew_engine_with_the_version_of_its_cellar_folder() {
    // Given
    let home = Home::complete();
    let installed = home
        .path()
        .join("Cellar/whisper-cpp/1.7.6/bin")
        .join(WHISPER);
    touch(&installed);
    let linked = home.path().join("bin").join(WHISPER);
    fs::remove_file(&linked).unwrap();
    std::os::unix::fs::symlink(&installed, &linked).unwrap();
    // When
    let origin = engine_origin(&linked, &home.machine());
    // Then
    assert_eq!(origin.to_string(), "whisper-server 1.7.6 from $PATH");
}

#[test]
pub(super) fn should_name_an_engine_of_unknown_version_by_where_it_came_from() {
    // Given
    let home = Home::complete();
    let chosen = home.path().join("elsewhere").join(WHISPER);
    touch(&chosen);
    let built = home.path().join("app").join(COMMAND_SPOTTING);
    touch(&built);
    let mut machine = home.machine();
    machine.env.insert(
        "VOICEPUNCHER_WHISPER_BINARY".into(),
        chosen.display().to_string(),
    );
    // When
    let from_env = engine_origin(&chosen, &machine);
    let from_build = engine_origin(&built, &machine);
    // Then
    assert_eq!(
        from_env.to_string(),
        "whisper-server (version unknown) from VOICEPUNCHER_WHISPER_BINARY"
    );
    assert_eq!(
        from_build.to_string(),
        "voicepuncher-command-spotting (version unknown) from the build folder"
    );
}
