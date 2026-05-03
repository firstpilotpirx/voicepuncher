use std::path::PathBuf;

use clap::{ArgAction, Parser};

use crate::core::domain::HotKey;

#[derive(Parser, Debug)]
#[command(
    name = "voicepuncher",
    version,
    about = "Push-to-talk voice→text for macOS Apple Silicon. Fully offline."
)]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long, value_parser = clap::value_parser!(HotKey))]
    pub hotkey: Option<HotKey>,

    #[arg(long)]
    pub whisper_model: Option<PathBuf>,

    #[arg(long)]
    pub llm_model: Option<PathBuf>,

    #[arg(long)]
    pub debug_save_wav: Option<PathBuf>,

    #[arg(
        long,
        value_name = "FILE",
        help = "Transcribe a WAV file and print to stdout (no hotkey)"
    )]
    pub input_file: Option<PathBuf>,

    #[arg(long, help = "Re-run the setup wizard even if config already exists")]
    pub setup: bool,

    #[arg(long, help = "Skip LLM post-processing; paste raw Whisper output")]
    pub no_llm: bool,

    #[arg(short = 'v', long, action = ArgAction::Count)]
    pub verbose: u8,
}

impl Cli {
    pub fn overlay(self, cfg: &mut crate::config::Config) {
        if let Some(key) = self.hotkey {
            cfg.hotkey.key = key;
        }
        if let Some(path) = self.whisper_model {
            cfg.whisper.model_path = path;
        }
        if let Some(path) = self.llm_model {
            cfg.llm.model_path = path;
        }
        if self.debug_save_wav.is_some() {
            cfg.audio.debug_save_wav = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::domain::HotKey;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn should_parse_with_all_defaults_when_no_args_given() {
        // Given / When
        let cli = Cli::parse_from(["voicepuncher"]);
        // Then
        assert!(cli.config.is_none());
        assert!(cli.hotkey.is_none());
        assert!(cli.whisper_model.is_none());
        assert!(cli.llm_model.is_none());
        assert!(cli.debug_save_wav.is_none());
        assert_eq!(cli.verbose, 0);
    }

    #[test]
    fn should_parse_hotkey_override() {
        // Given / When
        let cli = Cli::parse_from(["voicepuncher", "--hotkey", "F19"]);
        // Then
        assert_eq!(cli.hotkey, Some(HotKey::F19));
    }

    #[test]
    fn should_count_verbose_flag_repetitions() {
        // Given / When
        let cli = Cli::parse_from(["voicepuncher", "-vv"]);
        // Then
        assert_eq!(cli.verbose, 2);
    }

    #[test]
    fn should_overlay_cli_fields_onto_default_config() {
        // Given
        let cli = Cli {
            config: None,
            hotkey: Some(HotKey::F18),
            whisper_model: Some(PathBuf::from("/tmp/w.bin")),
            llm_model: Some(PathBuf::from("/tmp/l.gguf")),
            debug_save_wav: Some(PathBuf::from("/tmp/debug.wav")),
            input_file: None,
            setup: false,
            no_llm: false,
            verbose: 0,
        };
        let mut cfg = Config::default();
        // When
        cli.overlay(&mut cfg);
        // Then
        assert_eq!(cfg.hotkey.key, HotKey::F18);
        assert_eq!(cfg.whisper.model_path, PathBuf::from("/tmp/w.bin"));
        assert_eq!(cfg.llm.model_path, PathBuf::from("/tmp/l.gguf"));
        assert!(cfg.audio.debug_save_wav);
    }

    #[test]
    fn should_parse_input_file_flag() {
        // Given / When
        let cli = Cli::parse_from(["voicepuncher", "--input-file", "/tmp/test.wav"]);
        // Then
        assert_eq!(cli.input_file, Some(PathBuf::from("/tmp/test.wav")));
    }

    #[test]
    fn should_parse_no_llm_flag() {
        // Given / When
        let cli = Cli::parse_from(["voicepuncher", "--no-llm"]);
        // Then
        assert!(cli.no_llm);
    }
}
