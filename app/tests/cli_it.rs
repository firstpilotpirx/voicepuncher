use clap::Parser;
use std::path::PathBuf;
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
        "--llm-model",
        "/tmp/llm.gguf",
        "--debug-save-wav",
        "/tmp/debug.wav",
        "--no-llm",
    ]);
    let no_llm = cli.no_llm;
    let mut cfg = Config::default();

    cli.overlay(&mut cfg);

    assert_eq!(cfg.hotkey.key, HotKey::F19);
    assert_eq!(cfg.transcription.base_url, "http://localhost:9000/v1");
    assert_eq!(cfg.llm.model_path, PathBuf::from("/tmp/llm.gguf"));
    assert!(cfg.audio.debug_save_wav);
    assert!(no_llm);
}
