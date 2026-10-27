use std::path::PathBuf;

use crate::core::application::SoundPlayer;

mod loaded;
use loaded::LoadedSounds;

pub struct MacosSoundPlayer {
    start: Option<PathBuf>,
    stop: Option<PathBuf>,
    cancel: Option<PathBuf>,
    listening_on: Option<PathBuf>,
    listening_off: Option<PathBuf>,
    enter: Option<PathBuf>,
    clear: Option<PathBuf>,
    paste: Option<PathBuf>,
    restart: Option<PathBuf>,
    loaded: LoadedSounds,
}

impl MacosSoundPlayer {
    pub fn new(start_sound: &str, stop_sound: &str, cancel_sound: &str) -> Self {
        let loaded = LoadedSounds::start();
        Self {
            start: Self::resolve_loaded(start_sound, &loaded),
            stop: Self::resolve_loaded(stop_sound, &loaded),
            cancel: Self::resolve_loaded(cancel_sound, &loaded),
            listening_on: None,
            listening_off: None,
            enter: None,
            clear: None,
            paste: None,
            restart: None,
            loaded,
        }
    }

    /// Adds the sounds of voice control listening: on, off.
    pub fn with_voice_control_sounds(
        mut self,
        listening_on_sound: &str,
        listening_off_sound: &str,
    ) -> Self {
        self.listening_on = Self::resolve_loaded(listening_on_sound, &self.loaded);
        self.listening_off = Self::resolve_loaded(listening_off_sound, &self.loaded);
        self
    }

    /// Adds the sounds of the confirmed voice commands: enter, clear, paste, restart.
    pub fn with_command_sounds(
        mut self,
        enter_sound: &str,
        clear_sound: &str,
        paste_sound: &str,
        restart_sound: &str,
    ) -> Self {
        self.enter = Self::resolve_loaded(enter_sound, &self.loaded);
        self.clear = Self::resolve_loaded(clear_sound, &self.loaded);
        self.paste = Self::resolve_loaded(paste_sound, &self.loaded);
        self.restart = Self::resolve_loaded(restart_sound, &self.loaded);
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

    /// The file of the sound `name`, loaded ahead of its first play.
    fn resolve_loaded(name: &str, loaded: &LoadedSounds) -> Option<PathBuf> {
        let path = Self::resolve(name)?;
        loaded.load(&path);
        Some(path)
    }

    fn play_async(path: &std::path::Path) {
        match std::process::Command::new("afplay").arg(path).spawn() {
            Ok(mut child) => {
                log::debug!("sound: playing {}", path.display());
                // Reaped when it ends, so no zombie stays behind a sound.
                std::thread::spawn(move || child.wait());
            }
            Err(e) => log::warn!("sound: cannot play {}: {e}", path.display()),
        }
    }

    fn play_if_set(&self, path: Option<&PathBuf>) {
        if let Some(path) = path {
            self.loaded.play(path);
        }
    }
}

impl SoundPlayer for MacosSoundPlayer {
    fn play_start(&self) {
        self.play_if_set(self.start.as_ref());
    }

    fn play_stop(&self) {
        self.play_if_set(self.stop.as_ref());
    }

    fn play_cancel(&self) {
        self.play_if_set(self.cancel.as_ref());
    }

    fn play_listening_on(&self) {
        self.play_if_set(self.listening_on.as_ref());
    }

    fn play_listening_off(&self) {
        self.play_if_set(self.listening_off.as_ref());
    }

    fn play_enter(&self) {
        self.play_if_set(self.enter.as_ref());
    }

    fn play_clear(&self) {
        self.play_if_set(self.clear.as_ref());
    }

    fn play_paste(&self) {
        self.play_if_set(self.paste.as_ref());
    }

    fn play_restart(&self) {
        self.play_if_set(self.restart.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::SoundPlayer;

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

    #[test]
    fn should_play_nothing_when_both_sounds_are_empty() {
        // Given
        let player = MacosSoundPlayer::new("", "", "");
        // When
        player.play_start();
        player.play_stop();
        // Then
        assert!(player.start.is_none() && player.stop.is_none());
    }

    #[test]
    fn should_load_every_sound_of_the_default_configuration() {
        // Given
        let sounds = crate::config::SoundsCfg::default();
        let names = [
            sounds.start_sound,
            sounds.stop_sound,
            sounds.cancel_sound,
            sounds.listening_on_sound,
            sounds.listening_off_sound,
            sounds.enter_sound,
            sounds.clear_sound,
            sounds.paste_sound,
            sounds.restart_sound,
        ];
        // When
        let unloaded: Vec<&String> = names
            .iter()
            .filter(|name| {
                MacosSoundPlayer::resolve(name)
                    .and_then(|path| loaded::load(&path))
                    .is_none()
            })
            .collect();
        // Then
        assert!(
            unloaded.is_empty(),
            "these sounds did not load: {unloaded:?}"
        );
    }
}
