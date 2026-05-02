use std::path::{Path, PathBuf};

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

use crate::adapters::secondary::model_downloader::ModelDownloader;
use crate::core::domain::{HotKey, RecordingMode};

struct ModelSpec {
    label: &'static str,
    url: &'static str,
    filename: &'static str,
}

const WHISPER_MODELS: &[ModelSpec] = &[
    ModelSpec {
        label: "large-v3-turbo — multilingual, 1.6 GB  (recommended: best quality/speed)",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        filename: "ggml-large-v3-turbo.bin",
    },
    ModelSpec {
        label: "small          — multilingual, 460 MB",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        filename: "ggml-small.bin",
    },
    ModelSpec {
        label: "medium         — multilingual, 1.5 GB",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        filename: "ggml-medium.bin",
    },
    ModelSpec {
        label: "large-v3       — multilingual, 3.1 GB  (highest accuracy)",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        filename: "ggml-large-v3.bin",
    },
];

const LLM_MODELS: &[ModelSpec] = &[
    ModelSpec {
        label: "Qwen2.5-1.5B Q4_K_M — ~1 GB, fast  (recommended)",
        url: "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
        filename: "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf",
    },
    ModelSpec {
        label: "Qwen2.5-3B  Q4_K_M — ~2 GB, better quality",
        url: "https://huggingface.co/bartowski/Qwen2.5-3B-Instruct-GGUF/resolve/main/Qwen2.5-3B-Instruct-Q4_K_M.gguf",
        filename: "Qwen2.5-3B-Instruct-Q4_K_M.gguf",
    },
    ModelSpec {
        label: "Qwen2.5-7B  Q4_K_M — ~4.7 GB, best quality",
        url: "https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/main/Qwen2.5-7B-Instruct-Q4_K_M.gguf",
        filename: "Qwen2.5-7B-Instruct-Q4_K_M.gguf",
    },
];

const HOTKEY_OPTIONS: &[(&str, HotKey)] = &[
    (
        "RightOption   (recommended — right ⌥ key)",
        HotKey::RightOption,
    ),
    ("RightControl  (right ⌃)", HotKey::RightControl),
    ("RightShift    (right ⇧)", HotKey::RightShift),
    ("RightCommand  (right ⌘)", HotKey::RightCommand),
    ("LeftOption    (left ⌥)", HotKey::LeftOption),
    ("LeftControl   (left ⌃)", HotKey::LeftControl),
    ("Fn", HotKey::Fn),
    ("CapsLock", HotKey::CapsLock),
    ("F13", HotKey::F13),
    ("F14", HotKey::F14),
    ("F15", HotKey::F15),
    ("F16", HotKey::F16),
    ("F17", HotKey::F17),
    ("F18", HotKey::F18),
    ("F19", HotKey::F19),
];

const RECORDING_MODE_OPTIONS: &[(&str, RecordingMode)] = &[
    (
        "Toggle — press once to start, press again to stop  (recommended)",
        RecordingMode::Toggle,
    ),
    (
        "Hold   — hold key while speaking, release to stop",
        RecordingMode::Hold,
    ),
];

const DEFAULT_SYSTEM_PROMPT: &str =
    "You are a text cleanup assistant. Fix grammar, punctuation and capitalisation. \
     Output only the cleaned text, nothing else.";

pub struct SetupWizard;

impl SetupWizard {
    pub fn run_if_needed(config_path: &Path, force: bool) -> Result<()> {
        if !force && config_path.exists() {
            return Ok(());
        }
        Self::run(config_path)
    }

    fn run(config_path: &Path) -> Result<()> {
        let theme = ColorfulTheme::default();

        println!();
        println!("  voicepuncher — first-run setup");
        println!("  ─────────────────────────────────────");
        println!();

        let default_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".voicepuncher");

        let data_dir_str: String = Input::with_theme(&theme)
            .with_prompt("Models directory")
            .default(default_dir.to_string_lossy().to_string())
            .interact_text()?;
        let data_dir = PathBuf::from(data_dir_str);

        let whisper_idx = Select::with_theme(&theme)
            .with_prompt("Whisper model")
            .items(&WHISPER_MODELS.iter().map(|m| m.label).collect::<Vec<_>>())
            .default(0)
            .interact()?;

        let llm_idx = Select::with_theme(&theme)
            .with_prompt("LLM model")
            .items(&LLM_MODELS.iter().map(|m| m.label).collect::<Vec<_>>())
            .default(0)
            .interact()?;

        let lang: String = Input::with_theme(&theme)
            .with_prompt("Whisper language (ru / en / auto / …)")
            .default("ru".to_string())
            .interact_text()?;

        let hotkey_idx = Select::with_theme(&theme)
            .with_prompt("Hotkey")
            .items(&HOTKEY_OPTIONS.iter().map(|(l, _)| *l).collect::<Vec<_>>())
            .default(0)
            .interact()?;

        let mode_idx = Select::with_theme(&theme)
            .with_prompt("Recording mode")
            .items(
                &RECORDING_MODE_OPTIONS
                    .iter()
                    .map(|(l, _)| *l)
                    .collect::<Vec<_>>(),
            )
            .default(0)
            .interact()?;

        let whisper = &WHISPER_MODELS[whisper_idx];
        let llm = &LLM_MODELS[llm_idx];
        let hotkey = HOTKEY_OPTIONS[hotkey_idx].1;
        let recording_mode = RECORDING_MODE_OPTIONS[mode_idx].1;
        let whisper_path = data_dir.join(whisper.filename);
        let llm_path = data_dir.join(llm.filename);

        let do_download = Confirm::with_theme(&theme)
            .with_prompt("Download selected models now?")
            .default(true)
            .interact()?;

        if do_download {
            Self::download_if_missing(whisper.url, &whisper_path, "whisper")?;
            Self::download_if_missing(llm.url, &llm_path, "LLM")?;
        } else {
            println!();
            println!("  Skipping download. Place models at:");
            println!("    {}", whisper_path.display());
            println!("    {}", llm_path.display());
        }

        Self::write_config(
            config_path,
            &whisper_path,
            &llm_path,
            &lang,
            hotkey,
            recording_mode,
        )?;

        println!();
        println!("  Config written: {}", config_path.display());
        if do_download {
            println!("  Starting voicepuncher…");
        } else {
            println!("  Add models at the paths above, then run voicepuncher again.");
            std::process::exit(0);
        }
        println!();

        Ok(())
    }

    fn download_if_missing(url: &str, dest: &Path, label: &str) -> Result<()> {
        if dest.exists() {
            println!("  {label} model already present, skipping.");
            return Ok(());
        }
        println!();
        println!("  Downloading {label} model…");
        ModelDownloader::download(url, dest)
    }

    fn write_config(
        config_path: &Path,
        whisper_path: &Path,
        llm_path: &Path,
        lang: &str,
        hotkey: HotKey,
        recording_mode: RecordingMode,
    ) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = format!(
            "[hotkey]\n\
             key = \"{key}\"\n\
             min_press_ms = 200\n\
             recording_mode = \"{mode}\"\n\
             \n\
             [audio]\n\
             debug_save_wav = false\n\
             \n\
             [whisper]\n\
             model_path = \"{whisper}\"\n\
             language = \"{lang}\"\n\
             threads = 4\n\
             \n\
             [llm]\n\
             model_path = \"{llm}\"\n\
             max_tokens = 200\n\
             temperature = 0.0\n\
             context_size = 2048\n\
             system_prompt = \"{prompt}\"\n\
             \n\
             [paste]\n\
             clipboard_restore_delay_ms = 120\n\
             \n\
             [logging]\n\
             level = \"info\"\n",
            key = hotkey,
            mode = recording_mode,
            whisper = whisper_path.display(),
            lang = lang,
            llm = llm_path.display(),
            prompt = DEFAULT_SYSTEM_PROMPT,
        );
        std::fs::write(config_path, content)?;
        Ok(())
    }
}
