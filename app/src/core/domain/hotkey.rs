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
        self.names().human
    }

    /// The names of this hotkey: `NAMES` lists every hotkey in declaration order.
    fn names(self) -> &'static Names {
        &NAMES[self as usize]
    }
}

/// How a hotkey is written in the configuration and how a person reads it.
struct Names {
    key: HotKey,
    config: &'static str,
    human: &'static str,
}

const fn names(key: HotKey, config: &'static str, human: &'static str) -> Names {
    Names { key, config, human }
}

/// Every hotkey's names, in the order the enum declares them.
const NAMES: [Names; 18] = [
    names(HotKey::RightOption, "RightOption", "Right Option"),
    names(HotKey::RightControl, "RightControl", "Right Control"),
    names(HotKey::RightShift, "RightShift", "Right Shift"),
    names(HotKey::RightCommand, "RightCommand", "Right Command"),
    names(HotKey::LeftOption, "LeftOption", "Left Option"),
    names(HotKey::LeftControl, "LeftControl", "Left Control"),
    names(HotKey::Fn, "Fn", "Fn"),
    names(HotKey::CapsLock, "CapsLock", "Caps Lock"),
    names(HotKey::Space, "Space", "Space"),
    names(HotKey::CmdE, "CmdE", "Cmd+E"),
    names(HotKey::LeftOptionE, "LeftOptionE", "Option+E"),
    names(HotKey::F13, "F13", "F13"),
    names(HotKey::F14, "F14", "F14"),
    names(HotKey::F15, "F15", "F15"),
    names(HotKey::F16, "F16", "F16"),
    names(HotKey::F17, "F17", "F17"),
    names(HotKey::F18, "F18", "F18"),
    names(HotKey::F19, "F19", "F19"),
];

impl std::fmt::Display for HotKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.names().config)
    }
}

impl std::str::FromStr for HotKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        NAMES
            .iter()
            .find(|names| names.config == s)
            .map(|names| names.key)
            .ok_or_else(|| anyhow::anyhow!("unknown hotkey: {s}"))
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
    fn should_list_the_names_of_every_hotkey_at_its_own_place() {
        for key in HotKey::ALL {
            assert_eq!(key.names().key, key);
        }
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
