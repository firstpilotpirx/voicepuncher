use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::config::Config;
use crate::core::application::{ProbeAnswer, ServiceProbe};
use crate::core::domain::{ServiceKind, ServiceSpec};

/// Answers Ready on the given ports, Closed elsewhere.
struct Answering(Vec<u16>);

impl ServiceProbe for Answering {
    fn answer(&self, _kind: ServiceKind, port: u16) -> ProbeAnswer {
        if self.0.contains(&port) {
            ProbeAnswer::Ready
        } else {
            ProbeAnswer::Closed
        }
    }
}

const NOTHING_RUNS: Answering = Answering(Vec::new());

/// A home with every model in `~/.voicepuncher/models` and every engine in one `$PATH` folder.
struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    fn complete() -> Self {
        let home = Self::empty();
        let models = home.path().join(".voicepuncher/models");
        for file in [
            DICTATION_MODEL,
            COMMAND_MODEL,
            KWS_VAD_MODEL,
            "ggml-silero-v5.1.2.bin",
        ] {
            home.touch(&models.join(file));
        }
        fs::create_dir_all(models.join("sherpa-onnx-kws-zipformer")).unwrap();
        for engine in [WHISPER, COMMAND_SPOTTING] {
            home.touch(&home.path().join("bin").join(engine));
        }
        home
    }

    fn empty() -> Self {
        Self { dir: tempfile::tempdir().unwrap() }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn touch(&self, file: &Path) {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, b"").unwrap();
    }

    fn machine(&self) -> Machine {
        Machine {
            home: self.path().to_path_buf(),
            env: HashMap::new(),
            path: vec![self.path().join("bin")],
            app_dir: Some(self.path().join("app")),
            cores: 12,
        }
    }

    fn models(&self) -> PathBuf {
        self.path().join(".voicepuncher/models")
    }
}

fn kinds_and_ports(specs: &[ServiceSpec]) -> Vec<(ServiceKind, u16)> {
    specs.iter().map(|spec| (spec.kind, spec.port)).collect()
}

mod adoption;
mod command_lines;

const ALL: Needs = Needs { voice_control: true };

#[test]
fn should_need_only_the_dictation_copies_when_voice_control_is_off() {
    // Given
    let home = Home::complete();
    let needs = Needs { voice_control: false };
    // When
    let specs = needed(&Config::default(), needs, &home.machine(), &NOTHING_RUNS);
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
fn should_need_the_command_services_when_voice_control_is_on() {
    // Given
    let home = Home::complete();
    // When
    let specs = needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS);
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
fn should_run_one_copy_on_the_service_port_with_every_core() {
    // Given
    let home = Home::complete();
    let mut cfg = Config::default();
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
fn should_split_the_cores_between_the_copies() {
    assert_eq!(threads(12, 3), 4);
    assert_eq!(threads(2, 3), 1);
    assert_eq!(threads(8, 1), 8);
}

#[test]
fn should_run_one_configured_command_spotting_copy_on_the_service_port() {
    // Given
    let home = Home::complete();
    let mut cfg = Config::default();
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
fn should_label_each_command_spotting_copy() {
    let home = Home::complete();
    let specs = needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS);
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
fn should_start_each_command_spotting_copy_on_its_own_port() {
    // Given
    let home = Home::complete();
    let mut cfg = Config::default();
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
fn should_health_check_every_service_it_starts_each_copy_too() {
    // Given
    let home = Home::complete();
    // When
    let specs = needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let unchecked: Vec<&str> = specs
        .iter()
        .filter(|spec| !spec.health_checked)
        .map(|spec| spec.label.as_str())
        .collect();
    assert!(unchecked.is_empty(), "not health-checked: {unchecked:?}");
}

#[test]
fn should_log_each_service_to_a_file_named_by_its_program_and_port() {
    let home = Home::complete();
    let specs = needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS);
    let logs: Vec<PathBuf> = specs.iter().map(|spec| spec.log_file.clone()).collect();
    let in_logs = home.path().join(".voicepuncher/logs");
    assert!(logs.contains(&in_logs.join("whisper-server-18082.log")));
    assert!(logs.contains(&in_logs.join("voicepuncher-command-spotting-49083.log")));
}

#[test]
fn should_name_the_command_model_it_looked_for_when_it_is_not_on_disk() {
    // Given
    let home = Home::complete();
    let missing = home.path().join("elsewhere/ggml-commands.bin");
    let mut machine = home.machine();
    machine.env.insert(
        "VOICEPUNCHER_COMMAND_WHISPER_MODEL".into(),
        missing.display().to_string(),
    );
    // When
    let specs = needed(&Config::default(), ALL, &machine, &NOTHING_RUNS);
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
fn should_name_a_missing_engine_with_how_to_get_it() {
    // Given
    let home = Home::complete();
    fs::remove_file(home.path().join("bin").join(WHISPER)).unwrap();
    // When
    let specs = needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    let problem = specs[0].problem.clone().unwrap();
    assert!(problem.contains("whisper-server not found"), "{problem}");
    assert!(problem.contains("VOICEPUNCHER_WHISPER_BINARY"), "{problem}");
}

#[test]
fn should_find_an_engine_from_the_environment_then_the_bundle_then_the_path() {
    // Given
    let home = Home::complete();
    let bundled = home.path().join("app/engines/whisper/whisper-server");
    let chosen = home.path().join("elsewhere/whisper-server");
    home.touch(&bundled);
    home.touch(&chosen);
    let mut machine = home.machine();
    let lookup_program = |machine: &Machine| {
        needed(&Config::default(), ALL, machine, &NOTHING_RUNS)
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
fn should_take_every_engine_from_its_folder_in_the_bundle_before_the_path() {
    // Given
    let home = Home::complete();
    let engines = home.path().join("app/engines");
    for (folder, program) in [("whisper", WHISPER), ("command-spotting", COMMAND_SPOTTING)] {
        home.touch(&engines.join(folder).join(program));
    }
    // When
    let specs = needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS);
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
fn should_take_every_engine_from_the_path_outside_a_bundle() {
    // Given
    let home = Home::complete();
    // When
    let specs = needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS);
    // Then
    for spec in &specs {
        assert_eq!(
            spec.program.parent(),
            Some(home.path().join("bin").as_path())
        );
    }
}

#[test]
fn should_name_a_bundled_engine_with_the_version_of_its_folder() {
    // Given
    let home = Home::complete();
    let program = home.path().join("app/engines/whisper").join(WHISPER);
    home.touch(&program);
    fs::write(program.with_file_name("VERSION"), "1.9.5\n").unwrap();
    // When
    let origin = engine_origin(&program, &home.machine());
    // Then
    assert_eq!(origin.to_string(), "whisper-server 1.9.5 from the bundle");
}

#[test]
fn should_name_a_homebrew_engine_with_the_version_of_its_cellar_folder() {
    // Given
    let home = Home::complete();
    let installed = home
        .path()
        .join("Cellar/whisper-cpp/1.7.6/bin")
        .join(WHISPER);
    home.touch(&installed);
    let linked = home.path().join("bin").join(WHISPER);
    fs::remove_file(&linked).unwrap();
    std::os::unix::fs::symlink(&installed, &linked).unwrap();
    // When
    let origin = engine_origin(&linked, &home.machine());
    // Then
    assert_eq!(origin.to_string(), "whisper-server 1.7.6 from $PATH");
}

#[test]
fn should_name_an_engine_of_unknown_version_by_where_it_came_from() {
    // Given
    let home = Home::complete();
    let chosen = home.path().join("elsewhere").join(WHISPER);
    home.touch(&chosen);
    let built = home.path().join("app").join(COMMAND_SPOTTING);
    home.touch(&built);
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

/// The command spotting service as `needed` starts it on the home's machine.
fn spotting_spec(home: &Home) -> ServiceSpec {
    needed(&Config::default(), ALL, &home.machine(), &NOTHING_RUNS)
        .into_iter()
        .find(|spec| spec.kind == ServiceKind::CommandSpotting)
        .unwrap()
}

#[test]
fn should_find_the_command_spotting_service_built_next_to_the_app() {
    // Given
    let home = Home::complete();
    fs::remove_file(home.path().join("bin").join(COMMAND_SPOTTING)).unwrap();
    let built = home.path().join("app").join(COMMAND_SPOTTING);
    home.touch(&built);
    // When
    let spotting = spotting_spec(&home);
    // Then
    assert_eq!(spotting.program, built);
}

#[test]
fn should_find_a_model_in_the_old_folder_and_prefer_the_environment() {
    // Given
    let home = Home::complete();
    fs::remove_file(home.models().join(COMMAND_MODEL)).unwrap();
    let old = home.path().join(".voicepuncher").join(COMMAND_MODEL);
    home.touch(&old);
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
    home.touch(&chosen);
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
fn should_use_the_newest_of_two_keyword_spotting_models() {
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
