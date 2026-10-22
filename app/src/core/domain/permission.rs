use std::fmt;

/// A macOS permission voicepuncher needs for dictation by hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// To record the dictation.
    Microphone,
    /// To paste with a synthesized ⌘V and to read the focused text field.
    Accessibility,
    /// To notice the hotkey in any app.
    InputMonitoring,
}

impl Permission {
    pub const ALL: [Self; 3] = [Self::Microphone, Self::Accessibility, Self::InputMonitoring];

    /// The name System Settings uses for it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Microphone => "Microphone",
            Self::Accessibility => "Accessibility",
            Self::InputMonitoring => "Input Monitoring",
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Whether the system lets the app use a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    Granted,
    /// Denied or revoked by the user, or restricted.
    Denied,
    /// Never asked for: a request shows the system prompt.
    NotDetermined,
}

impl PermissionStatus {
    /// The status in words, as the log says it.
    pub fn words(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "not granted",
            Self::NotDetermined => "not asked yet",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_every_permission_as_system_settings_does() {
        let names: Vec<&str> = Permission::ALL.into_iter().map(Permission::name).collect();
        assert_eq!(names, ["Microphone", "Accessibility", "Input Monitoring"]);
    }

    #[test]
    fn should_say_every_status_in_words() {
        assert_eq!(PermissionStatus::Granted.words(), "granted");
        assert_eq!(PermissionStatus::Denied.words(), "not granted");
        assert_eq!(PermissionStatus::NotDetermined.words(), "not asked yet");
    }
}
