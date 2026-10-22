use std::sync::{Arc, Mutex};

/// The hotkey the listener matches key events against; the runtime rebinds it on a reload,
/// and every clone sees the new key at its next read.
#[derive(Debug, Clone)]
pub struct HotkeyBinding(Arc<Mutex<HotKey>>);

impl HotkeyBinding {
    pub fn new(hotkey: HotKey) -> Self {
        Self(Arc::new(Mutex::new(hotkey)))
    }

    pub fn current(&self) -> HotKey {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn rebind(&self, hotkey: HotKey) {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = hotkey;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum HotKey {
    RightOption,
    RightControl,
    RightShift,
    RightCommand,
    LeftOption,
    LeftControl,
    Fn,
    CapsLock,
    Space,
    CmdE,
    LeftOptionE,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
}

impl HotKey {
    /// Every hotkey the listener supports, in the order the settings window offers them.
    pub const ALL: [Self; 18] = [
        Self::RightOption,
        Self::RightControl,
        Self::RightShift,
        Self::RightCommand,
        Self::LeftOption,
        Self::LeftControl,
        Self::Fn,
        Self::CapsLock,
        Self::Space,
        Self::LeftOptionE,
        Self::CmdE,
        Self::F13,
        Self::F14,
        Self::F15,
        Self::F16,
        Self::F17,
        Self::F18,
        Self::F19,
    ];

    /// The hotkey as a person reads it: "Right Option", "Cmd+E".
    pub fn human_name(self) -> &'static str {
        match self {
            Self::RightOption => "Right Option",
            Self::RightControl => "Right Control",
            Self::RightShift => "Right Shift",
            Self::RightCommand => "Right Command",
            Self::LeftOption => "Left Option",
            Self::LeftControl => "Left Control",
            Self::Fn => "Fn",
            Self::CapsLock => "Caps Lock",
            Self::Space => "Space",
            Self::CmdE => "Cmd+E",
            Self::LeftOptionE => "Option+E",
            Self::F13 => "F13",
            Self::F14 => "F14",
            Self::F15 => "F15",
            Self::F16 => "F16",
            Self::F17 => "F17",
            Self::F18 => "F18",
            Self::F19 => "F19",
        }
    }
}

impl std::fmt::Display for HotKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::RightOption => "RightOption",
            Self::RightControl => "RightControl",
            Self::RightShift => "RightShift",
            Self::RightCommand => "RightCommand",
            Self::LeftOption => "LeftOption",
            Self::LeftControl => "LeftControl",
            Self::Fn => "Fn",
            Self::CapsLock => "CapsLock",
            Self::Space => "Space",
            Self::CmdE => "CmdE",
            Self::LeftOptionE => "LeftOptionE",
            Self::F13 => "F13",
            Self::F14 => "F14",
            Self::F15 => "F15",
            Self::F16 => "F16",
            Self::F17 => "F17",
            Self::F18 => "F18",
            Self::F19 => "F19",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for HotKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "RightOption" => Ok(Self::RightOption),
            "RightControl" => Ok(Self::RightControl),
            "RightShift" => Ok(Self::RightShift),
            "RightCommand" => Ok(Self::RightCommand),
            "LeftOption" => Ok(Self::LeftOption),
            "LeftControl" => Ok(Self::LeftControl),
            "Fn" => Ok(Self::Fn),
            "CapsLock" => Ok(Self::CapsLock),
            "Space" => Ok(Self::Space),
            "CmdE" => Ok(Self::CmdE),
            "LeftOptionE" => Ok(Self::LeftOptionE),
            "F13" => Ok(Self::F13),
            "F14" => Ok(Self::F14),
            "F15" => Ok(Self::F15),
            "F16" => Ok(Self::F16),
            "F17" => Ok(Self::F17),
            "F18" => Ok(Self::F18),
            "F19" => Ok(Self::F19),
            other => anyhow::bail!("unknown hotkey: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        key: HotKey,
    }

    #[test]
    fn should_read_the_hotkey_it_was_bound_with() {
        // Given
        let binding = HotkeyBinding::new(HotKey::RightOption);
        // When
        let current = binding.current();
        // Then
        assert_eq!(current, HotKey::RightOption);
    }

    #[test]
    fn should_show_a_rebind_to_every_clone_on_another_thread() {
        // Given
        let binding = HotkeyBinding::new(HotKey::RightOption);
        let reader = binding.clone();
        // When
        binding.rebind(HotKey::F19);
        let seen = std::thread::spawn(move || reader.current()).join().unwrap();
        // Then
        assert_eq!(seen, HotKey::F19);
    }

    #[test]
    fn should_list_every_hotkey_once_each_with_a_name_of_its_own() {
        // Given
        let mut names: Vec<&str> = HotKey::ALL.iter().map(|key| key.human_name()).collect();
        let mut parsed: Vec<HotKey> = HotKey::ALL
            .iter()
            .map(|key| key.to_string().parse().unwrap())
            .collect();
        // When
        names.sort_unstable();
        names.dedup();
        parsed.dedup();
        // Then
        assert_eq!(names.len(), HotKey::ALL.len());
        assert_eq!(parsed, HotKey::ALL);
    }

    #[test]
    fn should_parse_left_option_e_from_config_and_cli() {
        let wrapper: Wrapper = toml::from_str(r#"key = "LeftOptionE""#).unwrap();
        assert_eq!(wrapper.key, HotKey::LeftOptionE);
        assert_eq!("LeftOptionE".parse::<HotKey>().unwrap(), wrapper.key);
        assert_eq!(wrapper.key.to_string(), "LeftOptionE");
    }

    #[test]
    fn should_parse_right_option_from_str() {
        // Given
        let input = "RightOption";
        // When
        let result = input.parse::<HotKey>().unwrap();
        // Then
        assert_eq!(result, HotKey::RightOption);
    }

    #[test]
    fn should_parse_f19_from_str() {
        // Given
        let input = "F19";
        // When
        let result = input.parse::<HotKey>().unwrap();
        // Then
        assert_eq!(result, HotKey::F19);
    }

    #[test]
    fn should_fail_to_parse_unknown_key() {
        // Given
        let input = "Bogus";
        // When
        let result = input.parse::<HotKey>();
        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_deserialize_from_toml_string() {
        // Given
        let toml_input = r#"key = "RightOption""#;
        // When
        let wrapper: Wrapper = toml::from_str(toml_input).unwrap();
        // Then
        assert_eq!(wrapper.key, HotKey::RightOption);
    }
}
