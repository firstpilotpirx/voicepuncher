// Integration test for MacOsClaudeDictation.
// Reads the real frontmost app; the key presses go into the focused app and are watched
// through a listen-only event tap (needs Accessibility and Input Monitoring, a GUI session
// and the keyboard to itself): only `just test-real-keyboard` runs them, which sets
// VOICEPUNCHER_REAL_KEYBOARD_IT=1 (`needs_real_keyboard`).

use objc2_app_kit::NSWorkspace;

use voicepuncher::adapters::secondary::claudedictation::macos::{
    MacOsClaudeDictation, CLAUDE_BUNDLE_ID,
};
use voicepuncher::core::application::ClaudeDictation;
use voicepuncher::core::domain::KeyChord;
use voicepuncher_test_steps::required_env;

/// The key codes of D (with Command for the default shortcut) and Escape.
const KEY_D: i64 = 2;
const ESCAPE: i64 = 53;

fn claude() -> MacOsClaudeDictation {
    MacOsClaudeDictation::new(KeyChord::parse("cmd+d").unwrap())
}

#[test]
fn should_tell_claude_apart_by_the_real_frontmost_app() {
    // Given
    let frontmost = NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .and_then(|app| app.bundleIdentifier())
        .map(|id| id.to_string());
    // When
    let in_claude = claude().is_frontmost();
    // Then
    assert_eq!(
        in_claude,
        frontmost.as_deref() == Some(CLAUDE_BUNDLE_ID),
        "{frontmost:?}"
    );
}

mod needs_real_keyboard {
    use super::super::keypressing_macos::{keys_seen, watch_keys};
    use super::*;

    #[test]
    fn should_press_the_shortcut_then_escape() {
        // Given
        required_env("VOICEPUNCHER_REAL_KEYBOARD_IT");
        let keys = watch_keys();
        let mut claude = claude();
        // When
        claude.toggle().unwrap();
        claude.cancel().unwrap();
        // Then
        let seen = keys_seen(&keys, 3);
        assert!(
            seen.contains(&KEY_D) && seen.ends_with(&[ESCAPE]),
            "{seen:?}"
        );
    }
}
