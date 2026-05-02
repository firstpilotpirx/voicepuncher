// Integration test for MacOsTextPaster.
// Gated: set VOICEPUNCHER_REAL_PASTEBOARD=1 to run against the real macOS pasteboard.
// Skips cleanly when the env var is unset.

use voicepuncher::adapters::secondary::macos_text_paster::MacOsTextPaster;
use voicepuncher::core::application::ports::TextPaster;
use voicepuncher::core::domain::CleanedText;

#[test]
fn should_restore_clipboard_content_after_paste() {
    if std::env::var("VOICEPUNCHER_REAL_PASTEBOARD").is_err() {
        eprintln!("GATED: VOICEPUNCHER_REAL_PASTEBOARD not set, skipping");
        return;
    }

    // Given
    let prior = arboard::Clipboard::new().unwrap().get_text().ok();
    let mut paster = MacOsTextPaster::new(50);

    // When
    paster
        .paste(&CleanedText::new("voicepuncher-test-token-XYZ".to_string()).unwrap())
        .unwrap();

    // Then (immediate)
    let current = arboard::Clipboard::new().unwrap().get_text().unwrap();
    assert_eq!(current, "voicepuncher-test-token-XYZ");

    // Then (poll for restore within 130 ms ceiling)
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(130);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let current = arboard::Clipboard::new()
            .unwrap()
            .get_text()
            .unwrap_or_default();
        if current != "voicepuncher-test-token-XYZ" {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("clipboard was not restored within 130ms");
        }
    }

    let restored = arboard::Clipboard::new()
        .unwrap()
        .get_text()
        .unwrap_or_default();
    assert_eq!(restored, prior.unwrap_or_default());
}
