use super::*;

fn chord(text: &str) -> KeyChord {
    KeyChord::parse(text).unwrap()
}

fn with_frontmost(frontmost: fn() -> Option<String>) -> MacOsClaudeDictation {
    MacOsClaudeDictation { frontmost, ..MacOsClaudeDictation::new(chord("cmd+d")) }
}

#[test]
fn should_press_the_shortcut_key_with_command() {
    assert_eq!(
        MacOsClaudeDictation::new(chord("cmd+d")).shortcut,
        (KeyCode::ANSI_D, CGEventFlags::CGEventFlagCommand)
    );
    assert_eq!(
        MacOsClaudeDictation::new(chord("cmd+5")).shortcut,
        (KeyCode::ANSI_5, CGEventFlags::CGEventFlagCommand)
    );
}

#[test]
fn should_press_escape_without_a_modifier() {
    assert_eq!(ESCAPE, (KeyCode::ESCAPE, CGEventFlags::CGEventFlagNull));
}

#[test]
fn should_fail_with_accessibility_missing_when_not_trusted() {
    let mut claude =
        MacOsClaudeDictation { trusted: || false, ..MacOsClaudeDictation::new(chord("cmd+d")) };
    let toggled = claude.toggle().unwrap_err();
    let cancelled = claude.cancel().unwrap_err();
    assert!(toggled.is::<AccessibilityMissing>(), "{toggled:#}");
    assert!(cancelled.is::<AccessibilityMissing>(), "{cancelled:#}");
}

#[test]
fn should_count_only_the_claude_desktop_app_as_claude() {
    assert!(with_frontmost(|| Some("com.anthropic.claudefordesktop".to_owned())).is_frontmost());
    assert!(!with_frontmost(|| Some("com.apple.TextEdit".to_owned())).is_frontmost());
    assert!(!with_frontmost(|| None).is_frontmost());
}

#[test]
fn should_read_the_frontmost_app_on_a_test_thread() {
    // the answer depends on the desktop; reading it must not crash off the main thread
    let read = std::thread::spawn(frontmost_bundle_id).join();
    assert!(read.is_ok());
}
