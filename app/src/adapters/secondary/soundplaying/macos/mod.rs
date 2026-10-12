use std::path::PathBuf;

use crate::core::application::SoundPlayer;

pub struct MacosSoundPlayer {
    start_path: Option<PathBuf>,
    stop_path: Option<PathBuf>,
    cancel_path: Option<PathBuf>,
    listening_on_path: Option<PathBuf>,
    listening_off_path: Option<PathBuf>,
    command_path: Option<PathBuf>,
}

impl MacosSoundPlayer {
    pub fn new(start_sound: &str, stop_sound: &str, cancel_sound: &str) -> Self {
        Self {
            start_path: Self::resolve(start_sound),
            stop_path: Self::resolve(stop_sound),
            cancel_path: Self::resolve(cancel_sound),
            listening_on_path: None,
            listening_off_path: None,
            command_path: None,
        }
    }

    /// Adds the sounds of voice control: listening on, listening off, command carried out.
    pub fn with_voice_control_sounds(
        mut self,
        listening_on_sound: &str,
        listening_off_sound: &str,
        command_sound: &str,
    ) -> Self {
        self.listening_on_path = Self::resolve(listening_on_sound);
        self.listening_off_path = Self::resolve(listening_off_sound);
        self.command_path = Self::resolve(command_sound);
        self
    }

    pub fn preview(name: &str) {
        if let Some(path) = Self::resolve(name) {
            let _ = std::process::Command::new("afplay").arg(&path).status();
        }
    }

    pub fn preview_async(name: &str) -> Option<std::process::Child> {
        Self::resolve(name)
            .and_then(|path| std::process::Command::new("afplay").arg(path).spawn().ok())
    }

    fn resolve(name: &str) -> Option<PathBuf> {
        if name.is_empty() {
            return None;
        }
        let path = PathBuf::from(name);
        if path.is_absolute() {
            return Some(path);
        }
        let system_path = PathBuf::from(format!("/System/Library/Sounds/{name}.aiff"));
        if !system_path.exists() {
            log::warn!("system sound not found: {}", system_path.display());
        }
        Some(system_path)
    }

    fn play_async(path: &std::path::Path) {
        let _ = std::process::Command::new("afplay").arg(path).spawn();
    }

    fn play_if_set(path: &Option<PathBuf>) {
        if let Some(path) = path {
            Self::play_async(path);
        }
    }
}

impl SoundPlayer for MacosSoundPlayer {
    fn play_start(&self) {
        if let Some(ref path) = self.start_path {
            Self::play_async(path);
        }
    }

    fn play_stop(&self) {
        if let Some(ref path) = self.stop_path {
            Self::play_async(path);
        }
    }

    fn play_cancel(&self) {
        if let Some(ref path) = self.cancel_path {
            Self::play_async(path);
        }
    }

    fn play_listening_on(&self) {
        Self::play_if_set(&self.listening_on_path);
    }

    fn play_listening_off(&self) {
        Self::play_if_set(&self.listening_off_path);
    }

    fn play_command(&self) {
        Self::play_if_set(&self.command_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_resolve_empty_name_to_none() {
        // When
        let result = MacosSoundPlayer::resolve("");
        // Then
        assert!(result.is_none());
    }

    #[test]
    fn should_resolve_absolute_path_to_same_path() {
        // Given
        let absolute = "/tmp/test.aiff";
        // When
        let result = MacosSoundPlayer::resolve(absolute);
        // Then
        assert_eq!(result, Some(PathBuf::from(absolute)));
    }

    #[test]
    fn should_resolve_relative_name_to_system_sounds_path() {
        // Given
        let name = "Glass";
        // When
        let result = MacosSoundPlayer::resolve(name);
        // Then
        let path = result.expect("non-empty name must resolve to Some");
        assert_eq!(path, PathBuf::from("/System/Library/Sounds/Glass.aiff"));
    }

    #[test]
    fn should_resolve_name_with_spaces_to_system_sounds_path() {
        // Given — sound names on macOS may contain spaces
        let name = "My Sound";
        // When
        let result = MacosSoundPlayer::resolve(name);
        // Then
        let path = result.expect("non-empty name must resolve to Some");
        assert!(path.to_string_lossy().contains("My Sound"));
        assert!(path.to_string_lossy().ends_with(".aiff"));
    }
}
