//! The pill while recording and processing.

use super::{
    main_thread, overlay_with_notice_lasting, pump_run_loop, pump_until, Arc, AtomicU32,
    GlassFinish, MacosVisualIndicator, VisualIndicator, DRAWN_LIMIT, DRAWN_SECS, PASTED_GONE_SECS,
    SHORT_NOTICE,
};

pub(super) fn should_keep_the_recording_pill_when_the_screens_are_unchanged() {
    let mtm = main_thread();
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    indicator.show_recording();
    // When the window set is rebuilt twice over the same screens
    indicator.rebuild_window_set(mtm);
    indicator.rebuild_window_set(mtm);
    // Then
    pump_until(DRAWN_LIMIT, || indicator.first_shows_recording());
    assert!(indicator.is_pill_visible() && indicator.first_shows_recording());
}

pub(super) fn waveform_tick_cancels_on_hide() {
    let mtm = main_thread();

    // Given
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));

    // When
    indicator.show_recording();
    indicator.hide();

    // Then — hide() increments the generation guard; any scheduled tick sees a stale
    // generation and returns immediately without rescheduling
    assert!(
        indicator.generation_count() >= 2,
        "hide() must increment pulse_generation to at least 2"
    );

    // Drain the GCD main queue so any already-enqueued callbacks can fire;
    // the generation guard ensures they are no-ops
    pump_run_loop(0.2);
    assert!(!indicator.is_pill_visible(), "the hidden pill came back");
}

/// Silence draws dots this tall; speech draws taller columns.
pub(super) const DOT_HEIGHT: f64 = 2.0;

pub(super) fn should_raise_waveform_columns_for_a_loud_level() {
    // Given — RMS 0.5 is louder than the waveform's full scale
    let level_register = Arc::new(AtomicU32::new(0.5f32.to_bits()));
    let indicator = MacosVisualIndicator::initialize(main_thread(), Arc::clone(&level_register));

    // When — column periods of 200 ms pass until the overlay is drawn
    indicator.show_recording();
    pump_until(DRAWN_LIMIT, || {
        indicator.first_tallest_waveform_column() > DOT_HEIGHT
    });

    // Then
    let tallest = indicator.first_tallest_waveform_column();
    assert!(
        tallest > DOT_HEIGHT,
        "a loud level leaves a column taller than a dot, got {tallest}"
    );
    indicator.hide();
    pump_run_loop(DRAWN_SECS);
}

pub(super) fn should_draw_clear_glass_without_a_top_line() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recording();
    pump_run_loop(DRAWN_SECS);
    let finishes = overlay.first_glass_finishes();
    assert_eq!(finishes.len(), 2, "no pill and text window glass");
    finishes
        .iter()
        .for_each(|finish| assert_clear_glass_without_a_top_line(*finish));
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

/// On macOS 26 and later the system glass is clear and carries no rim of ours; the fallback
/// material keeps its rim.
pub(super) fn assert_clear_glass_without_a_top_line(finish: GlassFinish) {
    assert!(
        !finish.top_highlight,
        "a top line over the glass: {finish:?}"
    );
    match finish.clear_system_glass {
        Some(clear) => {
            assert!(clear, "the system glass is not clear: {finish:?}");
            assert!(!finish.own_rim, "our rim over the system glass: {finish:?}");
        }
        None => assert!(
            finish.own_rim,
            "the fallback glass lost its rim: {finish:?}"
        ),
    }
}

/// The pill and the text window are both drawn, and the text window rose in.
pub(super) const RISEN_SECS: f64 = 0.8;

pub(super) const LIVE_TRANSCRIPT: &str = "we moved the meeting to Friday";

pub(super) fn should_hang_the_live_transcript_below_the_pill() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recording();
    pump_run_loop(DRAWN_SECS);
    // When
    overlay
        .live_transcript_view()
        .show_live_transcript(LIVE_TRANSCRIPT);
    pump_run_loop(RISEN_SECS);
    // Then
    assert_eq!(
        overlay.first_text_window_value().as_deref(),
        Some(LIVE_TRANSCRIPT),
        "the text window does not hold the full live transcript"
    );
    let pill = overlay.first_pill_glass_frame().expect("no pill shows");
    let text = overlay
        .first_text_window_glass_frame()
        .expect("no text window shows");
    eprintln!("pill glass {pill:?}, text window glass {text:?}");
    let text_top = text.origin.y + text.size.height;
    assert!(
        text_top < pill.origin.y,
        "the text window (top {text_top}) reaches into the pill (bottom {})",
        pill.origin.y
    );
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

pub(super) fn should_pulse_the_record_dot_in_silence() {
    // Given — the level register holds silence
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    // When
    overlay.show_recording();
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(overlay.first_shows_motion(), "the record dot is still");
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

pub(super) fn should_keep_the_pill_size_from_the_recording_to_pasted() {
    // Given
    let level_register = Arc::new(AtomicU32::new(0));
    let overlay = MacosVisualIndicator::initialize_with_notice_duration(
        main_thread(),
        Arc::clone(&level_register),
        SHORT_NOTICE,
    );
    overlay.show_recording();
    pump_run_loop(DRAWN_SECS);
    let start = pill_size(&overlay, "at the start of the recording");
    // When — the recording hears speech, then the processing stages and Pasted follow
    level_register.store(0.5f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
    pump_run_loop(0.4);
    let speaking = pill_size(&overlay, "while recording with levels");
    overlay.show_recognizing_speech();
    pump_run_loop(DRAWN_SECS);
    let recognizing = pill_size(&overlay, "while recognizing");
    overlay.show_pasted();
    pump_run_loop(DRAWN_SECS);
    let pasted = pill_size(&overlay, "with Pasted");
    // Then
    assert_eq!(speaking, start, "the pill resized while recording");
    assert_eq!(recognizing, start, "the pill resized for recognizing");
    assert_eq!(pasted, start, "the pill resized for Pasted");
    pump_run_loop(PASTED_GONE_SECS);
}

/// The width and height of the pill's glass on the first screen.
pub(super) fn pill_size(overlay: &MacosVisualIndicator, moment: &str) -> (f64, f64) {
    let frame = overlay
        .first_pill_glass_frame()
        .unwrap_or_else(|| panic!("no pill shows {moment}"));
    (frame.size.width, frame.size.height)
}

pub(super) fn should_show_the_record_sign_and_waveform_without_a_label() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recording();
    // WHY: read in the frame the record sign appears, before the waveform's first tick
    pump_until(DRAWN_LIMIT, || overlay.first_shows_recording());
    assert!(overlay.first_shows_recording(), "no record sign");
    assert_eq!(
        overlay.first_label_value(),
        None,
        "the recording shows a label"
    );
    assert!(
        (overlay.first_tallest_waveform_column() - DOT_HEIGHT).abs() < 1e-6,
        "{} is not {}",
        overlay.first_tallest_waveform_column(),
        DOT_HEIGHT
    );
    assert!(!overlay.has_key_window(), "the glass takes the focus");
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

pub(super) fn should_show_each_processing_stage_with_its_icon_and_label() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recognizing_speech();
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_icon_symbol().as_deref(), Some("ear"));
    assert_eq!(overlay.first_label_value().as_deref(), Some("Recognizing"));
    assert_eq!(overlay.first_text_window_value(), None);
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

pub(super) fn should_show_the_wait_for_focus_without_a_text_window() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_waiting_for_focus();
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_icon_symbol().as_deref(), Some("keyboard"));
    assert_eq!(
        overlay.first_label_value().as_deref(),
        Some("Click a text field")
    );
    assert_eq!(overlay.first_text_window_value(), None);
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}
