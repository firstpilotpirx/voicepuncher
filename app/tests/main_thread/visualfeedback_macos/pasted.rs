//! The pasted sign, the text window and the rebuilds of the pill.

use super::{
    main_thread, overlay_with_notice_lasting, pump_run_loop, Arc, AtomicU32, MacosVisualIndicator,
    VisualIndicator, DRAWN_SECS, SHORT_NOTICE,
};

/// The Pasted sign shows 0.5 s and fades out in 0.2 s.
pub(super) const PASTED_GONE_SECS: f64 = 0.9;

pub(super) fn should_show_the_pasted_sign_and_hide_after_half_a_second() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    // When
    overlay.show_pasted();
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(overlay.is_pill_visible(), "the Pasted sign is not shown");
    assert_eq!(overlay.first_icon_symbol().as_deref(), Some("checkmark"));
    assert_eq!(overlay.first_label_value().as_deref(), Some("Pasted"));
    assert_eq!(overlay.first_text_window_value(), None);
    pump_run_loop(PASTED_GONE_SECS);
    assert!(!overlay.is_pill_visible(), "the Pasted sign did not hide");
    assert_eq!(overlay.first_text_window_value(), None);
}

pub(super) fn should_replace_the_pasted_sign_with_a_later_signal() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_pasted();
    pump_run_loop(DRAWN_SECS);
    // When
    overlay.show_recording();
    pump_run_loop(PASTED_GONE_SECS);
    // Then
    assert!(
        overlay.is_pill_visible(),
        "the Pasted timer hid the recording"
    );
    assert!(overlay.first_shows_recording());
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

pub(super) fn should_take_the_text_window_away_when_the_recording_ends() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recording();
    pump_run_loop(DRAWN_SECS);
    overlay
        .live_transcript_view()
        .show_live_transcript("we moved the meeting");
    pump_run_loop(DRAWN_SECS);
    assert!(
        overlay.first_text_window_value().is_some(),
        "no live transcript"
    );
    // When
    overlay.show_recognizing_speech();
    pump_run_loop(DRAWN_SECS);
    // Then
    assert_eq!(overlay.first_text_window_value(), None, "while recognizing");
    overlay.show_pasted();
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_text_window_value(), None, "with Pasted");
    pump_run_loop(PASTED_GONE_SECS);
}

pub(super) fn show_recording_survives_manual_rebuild() {
    let mtm = main_thread();
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    indicator.show_recording();
    pump_run_loop(0.2);
    indicator.rebuild_window_set(mtm);
    indicator.show_recording();
    pump_run_loop(0.2);
    assert!(
        indicator.generation_count() >= 2,
        "generation count should be at least 2 after show + rebuild + show"
    );
}

pub(super) fn show_recording_rebuilds_unconditionally() {
    let mtm = main_thread();
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));

    indicator.show_recording();
    pump_run_loop(0.2);
    let after_first = indicator.generation_count();

    indicator.show_recording();
    pump_run_loop(0.2);
    let after_second = indicator.generation_count();

    assert_eq!(
        after_first, 1,
        "first show_recording should bump generation to 1"
    );
    assert_eq!(
        after_second, 2,
        "second show_recording should bump generation to 2 (unconditional rebuild)"
    );
}

pub(super) fn show_recognizing_speech_rebuilds_unconditionally() {
    let mtm = main_thread();
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));

    indicator.show_recording();
    pump_run_loop(0.2);
    let after_recording = indicator.generation_count();

    indicator.show_recognizing_speech();
    pump_run_loop(0.2);
    let after_processing = indicator.generation_count();

    assert_eq!(
        after_recording, 1,
        "show_recording should bump generation to 1"
    );
    assert_eq!(
        after_processing, 2,
        "show_recognizing_speech should bump generation to 2 (unconditional rebuild)"
    );
}
