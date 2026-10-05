use std::path::Path;

use anyhow::Result;
use console::{Key, Term};
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::adapters::secondary::soundplaying::macos::MacosSoundPlayer;
use crate::config::{ensure_loopback, RefinementCfg, TranscriptionCfg};
use crate::core::domain::{HotKey, RecordingMode};

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
    ("CmdE         (⌘E chord)", HotKey::CmdE),
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

        let transcription_url: String = Input::with_theme(&theme)
            .with_prompt("Speech recognition service address")
            .default(TranscriptionCfg::default().base_url)
            .validate_with(|url: &String| {
                ensure_loopback("transcription.base_url", url).map_err(|e| e.to_string())
            })
            .interact_text()?;

        let refinement_url: String = Input::with_theme(&theme)
            .with_prompt("Text refinement service address")
            .default(RefinementCfg::default().base_url)
            .validate_with(|url: &String| {
                ensure_loopback("refinement.base_url", url).map_err(|e| e.to_string())
            })
            .interact_text()?;

        let refinement_model: String = Input::with_theme(&theme)
            .with_prompt("Text refinement model name (llama-server accepts any)")
            .default(RefinementCfg::default().model)
            .interact_text()?;

        let lang: String = Input::with_theme(&theme)
            .with_prompt("Spoken language (ru / en / auto / …)")
            .default("auto".to_string())
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

        let hotkey = HOTKEY_OPTIONS[hotkey_idx].1;
        let recording_mode = RECORDING_MODE_OPTIONS[mode_idx].1;

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

        Self::write_config(
            config_path,
            &SetupAnswers {
                transcription_url: &transcription_url,
                refinement_url: &refinement_url,
                refinement_model: &refinement_model,
                lang: &lang,
                hotkey,
                recording_mode,
                start_sound: &start_sound,
                stop_sound: &stop_sound,
            },
        )?;

        println!();
        println!("  Config written: {}", config_path.display());
        println!(
            "  Speech recognition and text refinement run in separate services — start them with 'just start-model-servers'."
        );
        println!("  Starting voicepuncher…");
        println!();

        Ok(())
    }

    /// Writes the answers as a config file, creating its directory if needed.
    pub fn write_config(config_path: &Path, p: &SetupAnswers<'_>) -> Result<()> {
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
             [transcription]\n\
             base_url = \"{transcription_url}\"\n\
             language = \"{lang}\"\n\
             \n\
             [refinement]\n\
             base_url = \"{refinement_url}\"\n\
             model = \"{refinement_model}\"\n\
             system_prompt = \"{prompt}\"\n\
             \n\
             [paste]\n\
             clipboard_restore_delay_ms = 120\n\
             focus_wait_seconds = 300\n\
             \n\
             [sounds]\n\
             start_sound = \"{start_sound}\"\n\
             stop_sound = \"{stop_sound}\"\n\
             \n\
             [logging]\n\
             level = \"info\"\n",
            key = p.hotkey,
            mode = p.recording_mode,
            transcription_url = p.transcription_url,
            lang = p.lang,
            refinement_url = p.refinement_url,
            refinement_model = p.refinement_model,
            prompt = RefinementCfg::default().system_prompt,
            start_sound = p.start_sound,
            stop_sound = p.stop_sound,
        );
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

pub struct SetupAnswers<'a> {
    pub transcription_url: &'a str,
    pub refinement_url: &'a str,
    pub refinement_model: &'a str,
    pub lang: &'a str,
    pub hotkey: HotKey,
    pub recording_mode: RecordingMode,
    pub start_sound: &'a str,
    pub stop_sound: &'a str,
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
