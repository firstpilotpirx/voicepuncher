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
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn rebind(&self, hotkey: HotKey) {
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = hotkey;
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
    pub const ALL: [HotKey; 18] = [
        HotKey::RightOption,
        HotKey::RightControl,
        HotKey::RightShift,
        HotKey::RightCommand,
        HotKey::LeftOption,
        HotKey::LeftControl,
        HotKey::Fn,
        HotKey::CapsLock,
        HotKey::Space,
        HotKey::LeftOptionE,
        HotKey::CmdE,
        HotKey::F13,
        HotKey::F14,
        HotKey::F15,
        HotKey::F16,
        HotKey::F17,
        HotKey::F18,
        HotKey::F19,
    ];

    /// The hotkey as a person reads it: "Right Option", "Cmd+E".
    pub fn human_name(self) -> &'static str {
        match self {
            HotKey::RightOption => "Right Option",
            HotKey::RightControl => "Right Control",
            HotKey::RightShift => "Right Shift",
            HotKey::RightCommand => "Right Command",
            HotKey::LeftOption => "Left Option",
            HotKey::LeftControl => "Left Control",
            HotKey::Fn => "Fn",
            HotKey::CapsLock => "Caps Lock",
            HotKey::Space => "Space",
            HotKey::CmdE => "Cmd+E",
            HotKey::LeftOptionE => "Option+E",
            HotKey::F13 => "F13",
            HotKey::F14 => "F14",
            HotKey::F15 => "F15",
            HotKey::F16 => "F16",
            HotKey::F17 => "F17",
            HotKey::F18 => "F18",
            HotKey::F19 => "F19",
        }
    }
}

impl std::fmt::Display for HotKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HotKey::RightOption => "RightOption",
            HotKey::RightControl => "RightControl",
            HotKey::RightShift => "RightShift",
            HotKey::RightCommand => "RightCommand",
            HotKey::LeftOption => "LeftOption",
            HotKey::LeftControl => "LeftControl",
            HotKey::Fn => "Fn",
            HotKey::CapsLock => "CapsLock",
            HotKey::Space => "Space",
            HotKey::CmdE => "CmdE",
            HotKey::LeftOptionE => "LeftOptionE",
            HotKey::F13 => "F13",
            HotKey::F14 => "F14",
            HotKey::F15 => "F15",
            HotKey::F16 => "F16",
            HotKey::F17 => "F17",
            HotKey::F18 => "F18",
            HotKey::F19 => "F19",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for HotKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "RightOption" => Ok(HotKey::RightOption),
            "RightControl" => Ok(HotKey::RightControl),
            "RightShift" => Ok(HotKey::RightShift),
            "RightCommand" => Ok(HotKey::RightCommand),
            "LeftOption" => Ok(HotKey::LeftOption),
            "LeftControl" => Ok(HotKey::LeftControl),
            "Fn" => Ok(HotKey::Fn),
            "CapsLock" => Ok(HotKey::CapsLock),
            "Space" => Ok(HotKey::Space),
            "CmdE" => Ok(HotKey::CmdE),
            "LeftOptionE" => Ok(HotKey::LeftOptionE),
            "F13" => Ok(HotKey::F13),
            "F14" => Ok(HotKey::F14),
            "F15" => Ok(HotKey::F15),
            "F16" => Ok(HotKey::F16),
            "F17" => Ok(HotKey::F17),
            "F18" => Ok(HotKey::F18),
            "F19" => Ok(HotKey::F19),
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
