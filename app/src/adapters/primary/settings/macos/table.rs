//! Every option of the configuration file, with its tab, label and control.

use super::field::{field, Choices, Control, Field, Scheme, Tab};

/// Options the loader still accepts so old files load, and the app ignores: no control.
pub const RETIRED: &[(&str, &str)] = &[("audio", "debug_save_wav")];

/// Keys the app writes itself and shows no control for.
pub const KEPT: &[(&str, &str)] = &[("app", "launch_at_login_asked")];

const MS: &str = "ms";
const SECONDS: &str = "s";
const MEGABYTES: &str = "MB";

const fn number(min: i64, max: i64, unit: &'static str) -> Control {
    Control::Number { min, max, unit }
}

const fn sound(key: &'static str, label: &'static str) -> Field {
    field(
        "sounds",
        key,
        Tab::Sounds,
        label,
        Control::PopUp(Choices::Sounds),
    )
}

/// Every option, in the order of its tab's rows.
pub const FIELDS: &[Field] = &[
    field(
        "hotkey",
        "key",
        Tab::General,
        "Hotkey",
        Control::PopUp(Choices::Hotkeys),
    ),
    field(
        "hotkey",
        "recording_mode",
        Tab::General,
        "Recording mode",
        Control::Radio(Choices::RecordingModes),
    ),
    field(
        "hotkey",
        "min_press_ms",
        Tab::General,
        "Minimum press",
        number(50, 1000, MS),
    ),
    field(
        "audio",
        "input_device",
        Tab::General,
        "Microphone",
        Control::PopUp(Choices::Microphones),
    ),
    field(
        "audio",
        "max_recording_seconds",
        Tab::General,
        "Longest recording",
        number(10, 3600, SECONDS),
    ),
    field(
        "voice_control",
        "enabled",
        Tab::General,
        "Voice control",
        Control::Switch,
    ),
    field(
        "voice_control",
        "url",
        Tab::General,
        "Voice control service",
        Control::Address(Scheme::WebSocket),
    )
    .advanced(),
    field(
        "services",
        "manage",
        Tab::Services,
        "Start the local services",
        Control::Switch,
    ),
    field(
        "services",
        "dictation_copies",
        Tab::Services,
        "Dictation service copies",
        number(1, 4, ""),
    ),
    field(
        "services",
        "command_copies",
        Tab::Services,
        "Command service copies",
        number(1, 4, ""),
    ),
    field(
        "services",
        "models_dir",
        Tab::Services,
        "Models folder",
        Control::Folder,
    ),
    field(
        "services",
        "commands_url",
        Tab::Services,
        "Command speech recognition service",
        Control::Address(Scheme::Http),
    )
    .advanced(),
    field(
        "transcription",
        "language",
        Tab::Dictation,
        "Language",
        Control::PopUp(Choices::Languages),
    ),
    field(
        "transcription",
        "languages",
        Tab::Dictation,
        "Languages to expect",
        Control::LanguageList,
    ),
    field(
        "transcription",
        "prompt",
        Tab::Dictation,
        "Prompt",
        Control::TextArea,
    ),
    field(
        "transcription",
        "temperature",
        Tab::Dictation,
        "Temperature",
        Control::Slider { min: 0.0, max: 1.0 },
    ),
    field(
        "transcription",
        "timeout_secs",
        Tab::Dictation,
        "Timeout",
        number(1, 600, SECONDS),
    ),
    field(
        "transcription",
        "chunk_threshold_secs",
        Tab::Dictation,
        "Chunk threshold",
        number(1, 600, SECONDS),
    )
    .advanced(),
    field(
        "transcription",
        "chunk_overlap_secs",
        Tab::Dictation,
        "Chunk overlap",
        number(0, 10, SECONDS),
    )
    .advanced(),
    field(
        "transcription",
        "vad_lookback_secs",
        Tab::Dictation,
        "VAD lookback",
        number(0, 30, SECONDS),
    )
    .advanced(),
    field(
        "transcription",
        "model",
        Tab::Dictation,
        "Model name",
        Control::Text,
    )
    .advanced(),
    field(
        "transcription",
        "base_url",
        Tab::Dictation,
        "Service address",
        Control::Address(Scheme::Http),
    )
    .advanced(),
    field(
        "refinement",
        "enabled",
        Tab::Refinement,
        "Refine text with the language model",
        Control::Switch,
    ),
    field(
        "refinement",
        "model",
        Tab::Refinement,
        "Model",
        Control::Text,
    ),
    field(
        "refinement",
        "system_prompt",
        Tab::Refinement,
        "System prompt",
        Control::TextArea,
    ),
    field(
        "refinement",
        "examples",
        Tab::Refinement,
        "Examples",
        Control::Examples,
    ),
    field(
        "refinement",
        "temperature",
        Tab::Refinement,
        "Temperature",
        Control::Slider { min: 0.0, max: 1.0 },
    ),
    field(
        "refinement",
        "max_tokens",
        Tab::Refinement,
        "Max tokens",
        number(16, 8192, ""),
    ),
    field(
        "refinement",
        "timeout_secs",
        Tab::Refinement,
        "Timeout",
        number(1, 600, SECONDS),
    ),
    field(
        "refinement",
        "base_url",
        Tab::Refinement,
        "Service address",
        Control::Address(Scheme::Http),
    )
    .advanced(),
    field(
        "refinement",
        "api_key",
        Tab::Refinement,
        "API key",
        Control::Text,
    )
    .advanced(),
    sound("start_sound", "Start"),
    sound("stop_sound", "Stop"),
    sound("cancel_sound", "Cancel"),
    sound("listening_on_sound", "Listening on"),
    sound("listening_off_sound", "Listening off"),
    sound("enter_sound", "Enter"),
    sound("clear_sound", "Clear"),
    sound("paste_sound", "Paste"),
    sound("restart_sound", "Restart"),
    field(
        "paste",
        "clipboard_restore_delay_ms",
        Tab::Paste,
        "Clipboard restore delay",
        number(0, 5000, MS),
    ),
    field(
        "paste",
        "focus_wait_seconds",
        Tab::Paste,
        "Wait for a text field",
        number(0, 3600, SECONDS),
    ),
    field(
        "archive",
        "enabled",
        Tab::Archive,
        "Keep every dictation",
        Control::Switch,
    ),
    field("archive", "dir", Tab::Archive, "Folder", Control::Folder),
    field(
        "archive",
        "max_size_mb",
        Tab::Archive,
        "Size limit",
        number(1, 1_000_000, MEGABYTES),
    ),
    field(
        "logging",
        "level",
        Tab::Logging,
        "Level",
        Control::PopUp(Choices::LogLevels),
    ),
];

/// The field of `section.key`.
pub fn find(section: &str, key: &str) -> Option<&'static Field> {
    FIELDS
        .iter()
        .find(|field| field.section == section && field.key == key)
}

#[cfg(test)]
mod tests;
