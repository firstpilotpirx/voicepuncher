use std::path::{Path, PathBuf};

use anyhow::Result;
use console::{Key, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

use crate::adapters::secondary::soundplaying::macos::MacosSoundPlayer;

use self::model_downloader::ModelDownloader;

pub mod model_downloader;
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

const SOUND_OPTIONS: &[&str] = &[
    "Glass",
    "Pop",
    "Ping",
    "Tink",
    "Bottle",
    "Frog",
    "Funk",
    "Hero",
    "Morse",
    "Purr",
    "Sosumi",
    "Submarine",
    "Basso",
    "Blow",
    "(none)",
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
    "You are a speech-to-text post-processor. Your ONLY job is to fix ASR recognition errors: \
     wrong words, technical terms transcribed in the wrong alphabet, \
     words accidentally merged or split. \
     CRITICAL RULES — NEVER BREAK THESE: \
     (1) NEVER shorten, cut, truncate, abbreviate, or summarize. \
     NEVER use ellipsis (...). NEVER omit any word. \
     Output the COMPLETE text — every single word, no exceptions. \
     (2) Do NOT rewrite, rephrase, restructure, or improve style. Fix only broken words. \
     (3) Preserve the original language — do not translate. \
     (4) Fix technical terms to their correct script \
     (e.g. Docker, API in English even inside Russian text). \
     (5) If nothing needs fixing, copy the input verbatim — every word, every number. \
     Output only the corrected text, nothing else.";

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

        let start_sound = sound_select(
            "Recording start sound",
            SOUND_OPTIONS,
            SOUND_OPTIONS
                .iter()
                .position(|&s| s == "Glass")
                .unwrap_or(0),
        )?;
        let stop_sound = sound_select(
            "Recording stop sound",
            SOUND_OPTIONS,
            SOUND_OPTIONS.iter().position(|&s| s == "Pop").unwrap_or(1),
        )?;

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
            WriteConfigParams {
                whisper_path: &whisper_path,
                llm_path: &llm_path,
                lang: &lang,
                hotkey,
                recording_mode,
                start_sound: &start_sound,
                stop_sound: &stop_sound,
            },
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

    fn write_config(config_path: &Path, p: WriteConfigParams<'_>) -> Result<()> {
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
             max_recording_seconds = 600\n\
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
             [sounds]\n\
             start_sound = \"{start_sound}\"\n\
             stop_sound = \"{stop_sound}\"\n\
             \n\
             [logging]\n\
             level = \"info\"\n",
            key = p.hotkey,
            mode = p.recording_mode,
            whisper = p.whisper_path.display(),
            lang = p.lang,
            llm = p.llm_path.display(),
            prompt = DEFAULT_SYSTEM_PROMPT,
            start_sound = p.start_sound,
            stop_sound = p.stop_sound,
        );
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

struct WriteConfigParams<'a> {
    whisper_path: &'a Path,
    llm_path: &'a Path,
    lang: &'a str,
    hotkey: HotKey,
    recording_mode: RecordingMode,
    start_sound: &'a str,
    stop_sound: &'a str,
}

fn sound_select(prompt: &str, options: &[&str], default_idx: usize) -> Result<String> {
    let term = Term::stderr();
    let n = options.len();
    let mut sel = default_idx;
    let mut child: Option<std::process::Child> = None;

    let draw = |term: &Term, sel: usize| -> Result<()> {
        term.write_line(&format!("? {prompt} (↑↓ to preview, Enter to confirm):"))?;
        for (i, &opt) in options.iter().enumerate() {
            if i == sel {
                term.write_line(&format!("  \x1b[36;1m❯ {opt}\x1b[0m"))?;
            } else {
                term.write_line(&format!("    {opt}"))?;
            }
        }
        Ok(())
    };

    draw(&term, sel)?;

    if options[sel] != "(none)" {
        child = MacosSoundPlayer::preview_async(options[sel]);
    }

    loop {
        match term.read_key()? {
            Key::ArrowUp => {
                sel = sel.saturating_sub(1);
            }
            Key::ArrowDown => {
                if sel < n - 1 {
                    sel += 1;
                }
            }
            Key::Enter => break,
            Key::Escape => {
                sel = default_idx;
                break;
            }
            _ => continue,
        }

        if let Some(mut c) = child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        if options[sel] != "(none)" {
            child = MacosSoundPlayer::preview_async(options[sel]);
        }

        term.clear_last_lines(n + 1)?;
        draw(&term, sel)?;
    }

    if let Some(mut c) = child.take() {
        let _ = c.kill();
        let _ = c.wait();
    }

    term.clear_last_lines(n + 1)?;
    let selected = options[sel];
    if selected == "(none)" {
        term.write_line(&format!("  {prompt} · (none)"))?;
        return Ok(String::new());
    }
    term.write_line(&format!("  {prompt} · {selected}"))?;

    Ok(selected.to_string())
}
