use std::path::PathBuf;

use crate::core::application::SoundPlayer;

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
}

impl MacosSoundPlayer {
    pub fn new(start_sound: &str, stop_sound: &str, cancel_sound: &str) -> Self {
        Self {
            start: Self::resolve(start_sound),
            stop: Self::resolve(stop_sound),
            cancel: Self::resolve(cancel_sound),
            listening_on: None,
            listening_off: None,
            enter: None,
            clear: None,
            paste: None,
            restart: None,
        }
    }

    /// Adds the sounds of voice control listening: on, off.
    pub fn with_voice_control_sounds(
        mut self,
        listening_on_sound: &str,
        listening_off_sound: &str,
    ) -> Self {
        self.listening_on = Self::resolve(listening_on_sound);
        self.listening_off = Self::resolve(listening_off_sound);
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
        self.enter = Self::resolve(enter_sound);
        self.clear = Self::resolve(clear_sound);
        self.paste = Self::resolve(paste_sound);
        self.restart = Self::resolve(restart_sound);
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
        match std::process::Command::new("afplay").arg(path).spawn() {
            Ok(mut child) => {
                log::debug!("sound: playing {}", path.display());
                // Reaped when it ends, so no zombie stays behind a sound.
                std::thread::spawn(move || child.wait());
            }
            Err(e) => log::warn!("sound: cannot play {}: {e}", path.display()),
        }
    }

    fn play_if_set(path: Option<&PathBuf>) {
        if let Some(path) = path {
            Self::play_async(path);
        }
    }
}

impl SoundPlayer for MacosSoundPlayer {
    fn play_start(&self) {
        if let Some(ref path) = self.start {
            Self::play_async(path);
        }
    }

    fn play_stop(&self) {
        if let Some(ref path) = self.stop {
            Self::play_async(path);
        }
    }

    fn play_cancel(&self) {
        if let Some(ref path) = self.cancel {
            Self::play_async(path);
        }
    }

    fn play_listening_on(&self) {
        Self::play_if_set(self.listening_on.as_ref());
    }

    fn play_listening_off(&self) {
        Self::play_if_set(self.listening_off.as_ref());
    }

    fn play_enter(&self) {
        Self::play_if_set(self.enter.as_ref());
    }

    fn play_clear(&self) {
        Self::play_if_set(self.clear.as_ref());
    }

    fn play_paste(&self) {
        Self::play_if_set(self.paste.as_ref());
    }

    fn play_restart(&self) {
        Self::play_if_set(self.restart.as_ref());
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
        assert_eq!(afplay_children(), Vec::<String>::new());
    }

    /// The children of this test process that still play or wait to be reaped: `afplay`, or a
    /// zombie, which `ps` names `<defunct>`.
    fn afplay_children() -> Vec<String> {
        let listing = std::process::Command::new("ps")
            .args(["-A", "-o", "ppid=,stat=,comm="])
            .output()
            .expect("ps runs");
        let own = std::process::id().to_string();
        String::from_utf8_lossy(&listing.stdout)
            .lines()
            .filter(|line| line.split_whitespace().next() == Some(own.as_str()))
            .filter(|line| line.ends_with("afplay") || line.ends_with("<defunct>"))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn should_leave_no_zombie_after_a_sound() {
        // Given
        let player = MacosSoundPlayer::new("Tink", "", "");
        // When — the short sound plays to its end
        player.play_start();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !afplay_children().is_empty() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        // Then — the ended afplay was reaped, not left as a zombie
        assert_eq!(afplay_children(), Vec::<String>::new());
    }
}
