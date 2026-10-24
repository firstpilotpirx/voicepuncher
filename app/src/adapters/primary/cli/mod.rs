use std::path::PathBuf;

use clap::Parser;

use crate::core::domain::HotKey;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "voicepuncher",
    version,
    about = "Push-to-talk voice→text for macOS Apple Silicon. Fully offline: talks only to local services."
)]
pub struct Cli {
    #[arg(
        long,
        value_name = "FILE",
        help = "The configuration file (default ~/.voicepuncher/config.toml; written with the defaults when missing)"
    )]
    pub config: Option<PathBuf>,

    #[arg(
        long,
        value_parser = clap::value_parser!(HotKey),
        help = "The recording hotkey; overrides the configuration for this run"
    )]
    pub hotkey: Option<HotKey>,

    #[arg(
        long,
        value_name = "URL",
        help = "Speech recognition service address; overrides the configuration for this run ([transcription].base_url)"
    )]
    pub transcription_url: Option<String>,

    #[arg(
        long,
        value_name = "DIR",
        help = "Keep each dictation in this folder, also in file mode; overrides the configuration for this run ([archive])"
    )]
    pub archive_dir: Option<PathBuf>,

    #[arg(
        long,
        help = "Keep no dictation archive (wins over --archive-dir); overrides the configuration for this run"
    )]
    pub no_archive: bool,

    #[arg(
        long,
        value_name = "FILE",
        help = "File mode: transcribe a WAV file and print the text to stdout (no hotkey, no windows)"
    )]
    pub input_file: Option<PathBuf>,

    #[arg(
        long,
        help = "Listen for spoken commands from the start, beside the hotkey; overrides the configuration for this run"
    )]
    pub voice_control: bool,

    #[arg(
        long,
        help = "Download the missing model files without a window and exit (0 when every file is present)"
    )]
    pub download_models: bool,

    #[arg(
        long,
        value_name = "DIR",
        help = "The models folder of --download-models; overrides the configuration for this run ([services].models_dir)"
    )]
    pub models_dir: Option<PathBuf>,
}

impl Cli {
    /// File mode keeps an archive only when a folder was named, so test runs never fill
    /// the user's archive.
    pub fn archives_file_mode(&self) -> bool {
        self.archive_dir.is_some() && !self.no_archive
    }

    /// Sets what the command line names over `cfg`; the runtime applies it again on every
    /// reload of the configuration file.
    pub fn overlay(&self, cfg: &mut crate::config::Config) {
        if let Some(key) = self.hotkey {
            cfg.hotkey.key = key;
        }
        if let Some(url) = &self.transcription_url {
            cfg.transcription.base_url = url.clone();
        }
        if let Some(dir) = &self.archive_dir {
            cfg.archive.dir = dir.clone();
            cfg.archive.enabled = true;
        }
        if self.no_archive {
            cfg.archive.enabled = false;
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
        assert!(cli.archive_dir.is_none());
        assert!(!cli.no_archive);
    }

    #[test]
    fn should_overlay_cli_fields_onto_default_config() {
        // Given
        let cli = Cli {
            config: None,
            hotkey: Some(HotKey::F18),
            transcription_url: Some("http://localhost:9000/v1".into()),
            archive_dir: Some(PathBuf::from("/tmp/dictations")),
            no_archive: false,
            input_file: None,
            voice_control: false,
            download_models: false,
            models_dir: None,
        };
        let mut cfg = Config::default();
        // When
        cli.overlay(&mut cfg);
        // Then
        assert_eq!(cfg.hotkey.key, HotKey::F18);
        assert_eq!(cfg.transcription.base_url, "http://localhost:9000/v1");
        assert_eq!(cfg.archive.dir, PathBuf::from("/tmp/dictations"));
    }

    #[test]
    fn should_parse_input_file_flag() {
        // Given / When
        let cli = Cli::parse_from(["voicepuncher", "--input-file", "/tmp/test.wav"]);
        // Then
        assert_eq!(cli.input_file, Some(PathBuf::from("/tmp/test.wav")));
    }

    #[test]
    fn should_turn_the_archive_on_in_the_named_folder_with_archive_dir() {
        // Given
        let cli = Cli::parse_from(["voicepuncher", "--archive-dir", "/tmp/dictations"]);
        let mut cfg = Config::default();
        cfg.archive.enabled = false;
        // When
        cli.overlay(&mut cfg);
        // Then
        assert!(cfg.archive.enabled);
        assert_eq!(cfg.archive.dir, PathBuf::from("/tmp/dictations"));
    }

    #[test]
    fn should_turn_the_archive_off_with_no_archive_even_with_archive_dir() {
        // Given
        let cli = Cli::parse_from([
            "voicepuncher",
            "--archive-dir",
            "/tmp/dictations",
            "--no-archive",
        ]);
        // When
        let archives_file_mode = cli.archives_file_mode();
        let mut cfg = Config::default();
        cli.overlay(&mut cfg);
        // Then
        assert!(!cfg.archive.enabled);
        assert!(!archives_file_mode);
    }

    #[test]
    fn should_archive_file_mode_only_with_archive_dir() {
        // Given
        let plain = Cli::parse_from(["voicepuncher", "--input-file", "/tmp/a.wav"]);
        let named = Cli::parse_from([
            "voicepuncher",
            "--input-file",
            "/tmp/a.wav",
            "--archive-dir",
            "/tmp/dictations",
        ]);
        // When / Then
        assert!(!plain.archives_file_mode());
        assert!(named.archives_file_mode());
    }

    #[test]
    fn should_reject_the_retired_debug_save_wav_flag() {
        // Given / When
        let result = Cli::try_parse_from(["voicepuncher", "--debug-save-wav", "/tmp/debug.wav"]);
        // Then
        assert!(result.is_err());
    }
}
