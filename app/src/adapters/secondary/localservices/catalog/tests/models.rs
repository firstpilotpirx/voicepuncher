//! Where the models and the command spotting service are found.

use super::*;

/// The command spotting service as `needed` starts it on the home's machine.
pub(super) fn spotting_spec(home: &Home) -> ServiceSpec {
    needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS)
        .into_iter()
        .find(|spec| spec.kind == ServiceKind::CommandSpotting)
        .unwrap()
}

#[test]
pub(super) fn should_find_the_command_spotting_service_built_next_to_the_app() {
    // Given
    let home = Home::complete();
    fs::remove_file(home.path().join("bin").join(COMMAND_SPOTTING)).unwrap();
    let built = home.path().join("app").join(COMMAND_SPOTTING);
    touch(&built);
    // When
    let spotting = spotting_spec(&home);
    // Then
    assert_eq!(spotting.program, built);
}

#[test]
pub(super) fn should_find_a_model_in_the_old_folder_and_prefer_the_environment() {
    // Given
    let home = Home::complete();
    fs::remove_file(home.models().join(COMMAND_MODEL)).unwrap();
    let old = home.path().join(".voicepuncher").join(COMMAND_MODEL);
    touch(&old);
    let mut machine = home.machine();
    let model_of = |machine: &Machine| {
        let specs = needed(&Config::default(), ALL, machine, &NOTHING_RUNS);
        let commands = specs
            .iter()
            .find(|spec| spec.kind == ServiceKind::Commands)
            .unwrap();
        commands.args[1].clone()
    };
    // When
    let found = model_of(&machine);
    let chosen = home.path().join("mine.bin");
    touch(&chosen);
    machine.env.insert(
        "VOICEPUNCHER_COMMAND_WHISPER_MODEL".into(),
        chosen.display().to_string(),
    );
    let overridden = model_of(&machine);
    // Then
    assert_eq!(found, old.display().to_string());
    assert_eq!(overridden, chosen.display().to_string());
}

#[test]
pub(super) fn should_use_the_newest_of_two_keyword_spotting_models() {
    // Given
    let home = Home::complete();
    let older = home.models().join("sherpa-onnx-kws-zipformer");
    let newer = home.models().join("sherpa-onnx-kws-zz-newer");
    fs::create_dir_all(&newer).unwrap();
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    fs::File::open(&older).unwrap().set_modified(past).unwrap();
    // When
    let spotting = spotting_spec(&home);
    // Then
    assert!(
        spotting.args.contains(&newer.display().to_string()),
        "{:?}",
        spotting.args
    );
}
