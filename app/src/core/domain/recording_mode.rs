#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum RecordingMode {
    Hold,
    #[default]
    Toggle,
}

impl std::fmt::Display for RecordingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hold => write!(f, "Hold"),
            Self::Toggle => write!(f, "Toggle"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_display_hold() {
        assert_eq!(RecordingMode::Hold.to_string(), "Hold");
    }

    #[test]
    fn should_display_toggle() {
        assert_eq!(RecordingMode::Toggle.to_string(), "Toggle");
    }

    #[test]
    fn should_deserialize_hold_from_toml() {
        #[derive(serde::Deserialize)]
        struct W {
            mode: RecordingMode,
        }
        let w: W = toml::from_str(r#"mode = "Hold""#).unwrap();
        assert_eq!(w.mode, RecordingMode::Hold);
    }

    #[test]
    fn should_deserialize_toggle_from_toml() {
        #[derive(serde::Deserialize)]
        struct W {
            mode: RecordingMode,
        }
        let w: W = toml::from_str(r#"mode = "Toggle""#).unwrap();
        assert_eq!(w.mode, RecordingMode::Toggle);
    }
}
