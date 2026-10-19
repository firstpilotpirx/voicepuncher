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
        "--refinement-url",
        "http://localhost:9090/v1",
        "--archive-dir",
        "/tmp/dictations",
        "--no-llm",
    ]);
    let no_llm = cli.no_llm;
    let mut cfg = Config::default();

    cli.overlay(&mut cfg);

    assert_eq!(cfg.hotkey.key, HotKey::F19);
    assert_eq!(cfg.transcription.base_url, "http://localhost:9000/v1");
    assert_eq!(cfg.refinement.base_url, "http://localhost:9090/v1");
    assert!(cfg.archive.enabled);
    assert_eq!(cfg.archive.dir, std::path::Path::new("/tmp/dictations"));
    assert!(no_llm);
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
