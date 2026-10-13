use std::path::PathBuf;

use clap::Parser;

use crate::core::domain::HotKey;

#[derive(Parser, Debug)]
#[command(
    name = "voicepuncher",
    version,
    about = "Push-to-talk voice→text for macOS Apple Silicon. Fully offline: talks only to local services."
)]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long, value_parser = clap::value_parser!(HotKey))]
    pub hotkey: Option<HotKey>,

    #[arg(
        long,
        value_name = "URL",
        help = "Speech recognition service address (overrides [transcription].base_url)"
    )]
    pub transcription_url: Option<String>,

    #[arg(
        long,
        value_name = "URL",
        help = "Text refinement service address (overrides [refinement].base_url)"
    )]
    pub refinement_url: Option<String>,

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

    #[arg(
        long,
        help = "Skip text refinement; paste the raw transcript without contacting the text refinement service"
    )]
    pub no_llm: bool,

    #[arg(
        long,
        help = "Voice control: listen for spoken commands from the start, alongside the push-to-talk hotkey"
    )]
    pub voice_control: bool,
}

impl Cli {
    pub fn overlay(self, cfg: &mut crate::config::Config) {
        if let Some(key) = self.hotkey {
            cfg.hotkey.key = key;
        }
        if let Some(url) = self.transcription_url {
            cfg.transcription.base_url = url;
        }
        if let Some(url) = self.refinement_url {
            cfg.refinement.base_url = url;
        }
        if self.debug_save_wav.is_some() {
            cfg.audio.debug_save_wav = true;
        }
        if self.voice_control {
            cfg.voice_control.enabled = true;
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
        assert!(cli.transcription_url.is_none());
        assert!(cli.refinement_url.is_none());
        assert!(cli.debug_save_wav.is_none());
    }

    #[test]
    fn should_overlay_cli_fields_onto_default_config() {
        // Given
        let cli = Cli {
            config: None,
            hotkey: Some(HotKey::F18),
            transcription_url: Some("http://localhost:9000/v1".into()),
            refinement_url: Some("http://localhost:9090/v1".into()),
            debug_save_wav: Some(PathBuf::from("/tmp/debug.wav")),
            input_file: None,
            setup: false,
            no_llm: false,
            voice_control: false,
        };
        let mut cfg = Config::default();
        // When
        cli.overlay(&mut cfg);
        // Then
        assert_eq!(cfg.hotkey.key, HotKey::F18);
        assert_eq!(cfg.transcription.base_url, "http://localhost:9000/v1");
        assert_eq!(cfg.refinement.base_url, "http://localhost:9090/v1");
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
