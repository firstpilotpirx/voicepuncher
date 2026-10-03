#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
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
