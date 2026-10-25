use clap::Parser;
use voicepuncher::adapters::primary::cli::Cli;
use voicepuncher::config::Config;
use voicepuncher::core::domain::HotKey;

#[test]
fn should_parse_and_overlay_all_modifiable_flags_onto_config() {
    let cli = Cli::parse_from([
        "voicepuncher",
        "--hotkey",
        "F19",
        "--transcription-url",
        "http://localhost:9000/v1",
        "--archive-dir",
        "/tmp/dictations",
    ]);
    let mut cfg = Config::default();

    cli.overlay(&mut cfg);

    assert_eq!(cfg.hotkey.key, HotKey::F19);
    assert_eq!(cfg.transcription.base_url, "http://localhost:9000/v1");
    assert!(cfg.archive.enabled);
    assert_eq!(cfg.archive.dir, std::path::Path::new("/tmp/dictations"));
}

#[test]
fn should_turn_voice_control_on_with_its_flag() {
    let cli = Cli::parse_from(["voicepuncher", "--voice-control"]);
    let mut cfg = Config::default();

    cli.overlay(&mut cfg);

    assert!(cfg.voice_control.enabled);
}

#[test]
fn should_turn_the_archive_off_with_its_flag() {
    let cli = Cli::parse_from(["voicepuncher", "--no-archive"]);
    let mut cfg = Config::default();

    cli.overlay(&mut cfg);

    assert!(!cfg.archive.enabled);
}

#[test]
fn should_reject_the_retired_debug_save_wav_flag() {
    let result = Cli::try_parse_from(["voicepuncher", "--debug-save-wav", "/tmp/debug.wav"]);

    assert!(result.is_err());
}

/// Runs `voicepuncher --download-models --models-dir <folder>` in a home of its own, with no
/// variable naming a model and every address on a port that refuses connections.
fn download_models(home: &std::path::Path, folder: &std::path::Path) -> std::process::Output {
    let refused = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_voicepuncher"));
    command
        .args(["--download-models", "--models-dir"])
        .arg(folder)
        .env("HOME", home)
        .env(
            "VOICEPUNCHER_MODEL_MIRROR",
            format!("http://127.0.0.1:{refused}"),
        );
    for file in voicepuncher::core::domain::CATALOG {
        command.env_remove(file.variable);
    }
    command.output().unwrap()
}

/// Puts every catalog file but `missing` in `folder` at its catalog size.
fn fill(folder: &std::path::Path, missing: Option<&str>) {
    use voicepuncher::core::domain::{ModelKind, CATALOG};
    std::fs::create_dir_all(folder).unwrap();
    for file in CATALOG.iter().filter(|file| Some(file.name) != missing) {
        match file.kind {
            ModelKind::File => std::fs::File::create(folder.join(file.name))
                .and_then(|placed| placed.set_len(file.size_bytes))
                .unwrap(),
            ModelKind::TarBz2 { folder: name } => {
                std::fs::create_dir_all(folder.join(name)).unwrap();
            }
        }
    }
}

#[test]
fn should_exit_0_and_fetch_nothing_when_every_model_is_present() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let folder = home.path().join("models");
    fill(&folder, None);
    // When
    let output = download_models(home.path(), &folder);
    // Then
    let printed = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{printed}");
    assert!(printed.contains("every model is present"), "{printed}");
}

#[test]
fn should_exit_1_naming_the_file_its_host_refused() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let folder = home.path().join("models");
    fill(&folder, Some("silero_vad.onnx"));
    // When
    let output = download_models(home.path(), &folder);
    // Then
    let printed = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{printed}");
    assert!(
        printed.contains("silero_vad.onnx: failed: no internet connection"),
        "{printed}"
    );
}

/// Runs the app with `args` in a home of its own, with nothing on its standard input.
fn run(home: &std::path::Path, args: &[&std::ffi::OsStr]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_voicepuncher"))
        .args(args)
        .env("HOME", home)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

/// An address on this machine where nothing listens.
fn refused_url() -> String {
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    format!("http://127.0.0.1:{port}/v1")
}

#[test]
fn should_refuse_the_retired_setup_flag_with_exit_2() {
    // Given
    let home = tempfile::tempdir().unwrap();
    // When
    let output = run(home.path(), &["--setup".as_ref()]);
    // Then
    let printed = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{printed}");
    assert!(printed.contains("--setup"), "{printed}");
}

#[test]
fn should_exit_2_naming_an_unknown_option() {
    // Given
    let home = tempfile::tempdir().unwrap();
    // When
    let output = run(home.path(), &["--no-such-option".as_ref()]);
    // Then
    let printed = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{printed}");
    assert!(printed.contains("--no-such-option"), "{printed}");
}

#[test]
fn should_refuse_the_retired_text_refinement_options_with_exit_2() {
    for option in ["--llm", "--no-llm"] {
        // Given
        let home = tempfile::tempdir().unwrap();
        // When
        let output = run(home.path(), &[option.as_ref()]);
        // Then
        let printed = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{option}: {printed}");
        assert!(printed.contains(option), "{printed}");
    }
}

#[test]
fn should_write_the_default_file_and_ask_nothing_at_a_first_launch() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("settings/config.toml");
    let recording = home.path().join("not-a-recording.wav");
    std::fs::write(&recording, "not a recording").unwrap();
    // When
    let output = transcribe_refused(home.path(), config.as_os_str(), &recording);
    // Then
    let printed = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        std::fs::read_to_string(&config).unwrap(),
        voicepuncher::config::DEFAULT_FILE
    );
    assert!(
        printed.contains(&format!(
            "config: created {} with the defaults",
            config.display()
        )),
        "{printed}"
    );
    assert_eq!(output.status.code(), Some(1), "{printed}");
    assert!(output.stdout.is_empty());
}

#[test]
fn should_exit_2_when_the_file_mode_service_does_not_answer() {
    // Given
    let home = tempfile::tempdir().unwrap();
    let recording = home.path().join("silence.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&recording, spec).unwrap();
    for _ in 0..16_000 {
        writer.write_sample(0i16).unwrap();
    }
    writer.finalize().unwrap();
    // When
    let output = transcribe_refused(home.path(), "/dev/null".as_ref(), &recording);
    // Then
    let printed = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{printed}");
    let errors = printed
        .lines()
        .filter(|line| line.contains("ERROR"))
        .count();
    assert_eq!(errors, 1, "one error line expected:\n{printed}");
}

/// Whether `arg` is a developer option whose help says nothing of overriding the configuration.
fn overrides_silently(arg: &clap::Arg) -> bool {
    let own = matches!(
        arg.get_long(),
        Some("config" | "input-file" | "download-models" | "help" | "version")
    );
    let says = arg.get_help().is_some_and(|help| {
        help.to_string()
            .contains("overrides the configuration for this run")
    });
    !own && !says
}

#[test]
fn should_offer_only_the_developer_options_each_saying_it_overrides_the_configuration() {
    // Given
    use clap::CommandFactory;
    let mut command = Cli::command();
    command.build();
    // When
    let options: Vec<String> = command
        .get_arguments()
        .filter_map(|arg| arg.get_long().map(str::to_owned))
        .collect();
    let silent: Vec<String> = command
        .get_arguments()
        .filter(|arg| overrides_silently(arg))
        .filter_map(|arg| arg.get_long().map(str::to_owned))
        .collect();
    // Then
    let mut sorted = options;
    sorted.sort();
    assert_eq!(
        sorted,
        [
            "archive-dir",
            "config",
            "download-models",
            "help",
            "hotkey",
            "input-file",
            "models-dir",
            "no-archive",
            "transcription-url",
            "version",
            "voice-control",
        ]
    );
    assert!(
        silent.is_empty(),
        "help says nothing of overriding: {silent:?}"
    );
}

#[test]
fn should_depend_on_no_terminal_prompt_library() {
    // Given
    let lock =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock")).unwrap();
    // When
    let prompts = lock.contains("name = \"dialoguer\"");
    // Then
    assert!(!prompts, "Cargo.lock still lists dialoguer");
}

/// Runs file mode on `recording` with the configuration file `config`, against a transcription
/// service that refuses every connection.
fn transcribe_refused(
    home: &std::path::Path,
    config: &std::ffi::OsStr,
    recording: &std::path::Path,
) -> std::process::Output {
    run(
        home,
        &[
            "--config".as_ref(),
            config,
            "--input-file".as_ref(),
            recording.as_os_str(),
            "--transcription-url".as_ref(),
            refused_url().as_ref(),
        ],
    )
}
