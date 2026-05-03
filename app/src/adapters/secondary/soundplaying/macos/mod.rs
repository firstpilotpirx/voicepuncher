use std::path::PathBuf;

use crate::core::application::SoundPlayer;

pub struct MacosSoundPlayer {
    start_path: Option<PathBuf>,
    stop_path: Option<PathBuf>,
}

impl MacosSoundPlayer {
    pub fn new(start_sound: &str, stop_sound: &str) -> Self {
        Self {
            start_path: Self::resolve(start_sound),
            stop_path: Self::resolve(stop_sound),
        }
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
