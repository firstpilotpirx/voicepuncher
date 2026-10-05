/// What the Accessibility API reports about the element that has keyboard focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusedElement {
    pub role: String,
    pub has_text_selection: bool,
    pub value_settable: bool,
}

/// Roles of controls that never take typed text. Any role not listed here may be a text
/// input in disguise (web content, Electron groups), so it never makes the app wait.
const NON_TEXT_ROLES: [&str; 13] = [
    "AXButton",
    "AXList",
    "AXTable",
    "AXOutline",
    "AXImage",
    "AXMenu",
    "AXMenuBar",
    "AXMenuItem",
    "AXCheckBox",
    "AXRadioButton",
    "AXSlider",
    "AXTabGroup",
    "AXToolbar",
];

impl FocusedElement {
    fn is_surely_not_text(&self) -> bool {
        NON_TEXT_ROLES.contains(&self.role.as_str())
            && !self.has_text_selection
            && !self.value_settable
    }
}

impl std::fmt::Display for FocusedElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.role)?;
        if self.has_text_selection {
            write!(f, " +selection")?;
        }
        if self.value_settable {
            write!(f, " +settable")?;
        }
        Ok(())
    }
}

/// Text input focus: whether the final text can be pasted now or the app waits for focus.
/// `seen` describes the focused element for the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFocus {
    Paste { seen: String },
    Wait { seen: String },
}

impl TextFocus {
    pub fn of(probed: Result<Option<&FocusedElement>, &anyhow::Error>) -> Self {
        match probed {
            Err(e) => TextFocus::Paste {
                seen: format!("unknown ({e})"),
            },
            Ok(None) => TextFocus::Paste {
                seen: "no focused element".to_owned(),
            },
            Ok(Some(el)) if el.is_surely_not_text() => TextFocus::Wait {
                seen: el.to_string(),
            },
            Ok(Some(el)) => TextFocus::Paste {
                seen: el.to_string(),
            },
        }
    }

    pub fn seen(&self) -> &str {
        match self {
            TextFocus::Paste { seen } | TextFocus::Wait { seen } => seen,
        }
    }

    pub fn waits(&self) -> bool {
        matches!(self, TextFocus::Wait { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(role: &str, has_text_selection: bool, value_settable: bool) -> FocusedElement {
        FocusedElement {
            role: role.to_owned(),
            has_text_selection,
            value_settable,
        }
    }

    fn verdict(el: &FocusedElement) -> TextFocus {
        TextFocus::of(Ok(Some(el)))
    }

    #[test]
    fn should_paste_into_text_roles() {
        for role in ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"] {
            assert!(!verdict(&element(role, true, true)).waits(), "{role}");
        }
    }

    #[test]
    fn should_paste_into_generic_roles() {
        for role in [
            "AXWebArea",
            "AXGroup",
            "AXWindow",
            "AXScrollArea",
            "AXUnknown",
        ] {
            assert!(!verdict(&element(role, false, false)).waits(), "{role}");
        }
    }

    #[test]
    fn should_paste_when_no_element_is_focused() {
        let focus = TextFocus::of(Ok(None));
        assert_eq!(
            focus,
            TextFocus::Paste {
                seen: "no focused element".into()
            }
        );
    }

    #[test]
    fn should_paste_when_focus_cannot_be_read() {
        let e = anyhow::anyhow!("AX error -25204");
        let focus = TextFocus::of(Err(&e));
        assert_eq!(
            focus,
            TextFocus::Paste {
                seen: "unknown (AX error -25204)".into()
            }
        );
    }

    #[test]
    fn should_wait_on_every_listed_non_text_role() {
        for role in NON_TEXT_ROLES_UNDER_TEST {
            assert!(verdict(&element(role, false, false)).waits(), "{role}");
        }
    }

    #[test]
    fn should_paste_into_a_listed_role_with_a_text_selection() {
        assert!(!verdict(&element("AXList", true, false)).waits());
    }

    #[test]
    fn should_paste_into_a_listed_role_with_a_settable_value() {
        assert!(!verdict(&element("AXButton", false, true)).waits());
    }

    #[test]
    fn should_describe_the_element_with_role_and_flags() {
        assert_eq!(
            verdict(&element("AXButton", false, false)),
            TextFocus::Wait {
                seen: "AXButton".into()
            }
        );
        assert_eq!(
            verdict(&element("AXTextArea", true, true)).seen(),
            "AXTextArea +selection +settable"
        );
    }

    const NON_TEXT_ROLES_UNDER_TEST: [&str; 13] = [
        "AXButton",
        "AXList",
        "AXTable",
        "AXOutline",
        "AXImage",
        "AXMenu",
        "AXMenuBar",
        "AXMenuItem",
        "AXCheckBox",
        "AXRadioButton",
        "AXSlider",
        "AXTabGroup",
        "AXToolbar",
    ];
}
