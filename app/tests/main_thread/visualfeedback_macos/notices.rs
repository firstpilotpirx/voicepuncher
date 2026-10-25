//! Failure notices and the commands unavailable sign.

use super::{
    main_thread, pump_run_loop, pump_until, pump_until_moment, Arc, AtomicU32, Duration,
    FailingPart, FailureNotice, Instant, MacosVisualIndicator, NSApplication, VisualIndicator,
};

/// A notice that lasts long enough for the tests to look at it, and short enough not to wait.
pub(super) const SHORT_NOTICE: Duration = Duration::from_millis(600);

/// Long enough for the main queue to draw what was asked.
pub(super) const DRAWN_SECS: f64 = 0.2;

/// `DRAWN_SECS` as a duration.
pub(super) const DRAWN_TIME: Duration = Duration::from_millis(200);

/// The longest a busy machine may take to draw what was asked.
pub(super) const DRAWN_LIMIT: Duration = Duration::from_secs(2);

/// Shorter than the waveform's 16 ms tick, so a step ends before the tick redraws.
pub(super) const PUMP_STEP_SECS: f64 = 0.001;

pub(super) const NOTICE: FailureNotice =
    FailureNotice::DictationLost(FailingPart::SpeechRecognition);

pub(super) fn overlay_with_notice_lasting(duration: Duration) -> MacosVisualIndicator {
    MacosVisualIndicator::initialize_with_notice_duration(
        main_thread(),
        Arc::new(AtomicU32::new(0)),
        duration,
    )
}

pub(super) fn should_show_the_notice_sign_without_a_text_window_or_activating() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    // When
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(overlay.is_pill_visible(), "the pill shows the notice");
    assert_eq!(
        overlay.first_icon_symbol().as_deref(),
        Some("exclamationmark.triangle.fill")
    );
    assert_eq!(
        overlay.first_label_value().as_deref(),
        Some(NOTICE.part_name())
    );
    assert_eq!(overlay.first_text_window_value(), None);
    assert!(
        !overlay.has_key_window(),
        "no overlay window takes the focus"
    );
    assert!(!NSApplication::sharedApplication(main_thread()).isActive());
}

pub(super) fn should_log_the_notice_message_as_a_warning() {
    // Given
    let lines = voicepuncher_test_steps::log_lines::captured(log::LevelFilter::Warn);
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    // When
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    // Then
    let warnings = lines.starting_with("");
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains(&NOTICE.message())),
        "the notice message {:?} is not logged: {warnings:?}",
        NOTICE.message()
    );
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

pub(super) fn should_hide_the_notice_after_its_time() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    assert!(
        overlay.is_pill_visible(),
        "the notice is shown before its time ends"
    );
    // When
    pump_run_loop(SHORT_NOTICE.as_secs_f64());
    // Then
    assert!(!overlay.is_pill_visible(), "the notice hid by itself");
    assert_eq!(overlay.first_text_window_value(), None);
}

/// A notice whose time leaves room to look between the first notice's end and the end of the
/// one shown again.
pub(super) const NOTICE_SHOWN_AGAIN: Duration = Duration::from_millis(1_500);

/// How many times the notice is shown again when the machine stalled past the moment to look.
pub(super) const LOOKS: usize = 5;

pub(super) fn should_restart_the_time_of_a_notice_shown_again() {
    for _ in 0..LOOKS {
        // Given
        let overlay = overlay_with_notice_lasting(NOTICE_SHOWN_AGAIN);
        let shown_at = Instant::now();
        overlay.show_failure_notice(&NOTICE);
        pump_until_moment(shown_at + NOTICE_SHOWN_AGAIN * 2 / 3);
        // When
        let shown_again_at = Instant::now();
        overlay.show_failure_notice(&NOTICE);
        // Then — past the first notice's end, before the end of the one shown again
        pump_until_moment(shown_at + NOTICE_SHOWN_AGAIN + DRAWN_TIME);
        let (visible, label) = (overlay.is_pill_visible(), overlay.first_label_value());
        if Instant::now() >= shown_again_at + NOTICE_SHOWN_AGAIN {
            overlay.hide();
            pump_run_loop(DRAWN_SECS);
            continue;
        }
        assert!(visible, "the first notice's time no longer counts");
        assert_eq!(label.as_deref(), Some(NOTICE.part_name()));
        pump_until(
            (shown_again_at + NOTICE_SHOWN_AGAIN + DRAWN_LIMIT)
                .saturating_duration_since(Instant::now()),
            || !overlay.is_pill_visible(),
        );
        assert!(
            !overlay.is_pill_visible(),
            "the notice shown again hid after its own time"
        );
        return;
    }
    panic!("the machine stalled past the moment to look {LOOKS} times");
}

pub(super) fn should_keep_a_recording_started_after_a_notice() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    assert_eq!(
        overlay.first_label_value().as_deref(),
        Some(NOTICE.part_name()),
        "the notice is shown"
    );
    // When
    overlay.show_recording();
    pump_run_loop(SHORT_NOTICE.as_secs_f64() + DRAWN_SECS);
    // Then
    assert!(
        overlay.is_pill_visible(),
        "the notice's timer left the recording shown"
    );
    assert!(overlay.first_shows_recording(), "the recording is shown");
    assert_eq!(overlay.first_text_window_value(), None);
}

pub(super) fn should_keep_the_commands_unavailable_sign_when_the_overlay_hides() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recording();
    overlay.show_commands_unavailable();
    pump_run_loop(DRAWN_SECS);
    assert!(overlay.first_shows_recording());
    assert!(overlay.first_shows_commands_unavailable());
    // When
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(overlay.is_pill_visible(), "the sign stays after the hide");
    assert!(overlay.first_shows_commands_unavailable());
    assert_eq!(overlay.first_label_value().as_deref(), Some("CMD"));
    assert!(
        !overlay.has_key_window(),
        "no overlay window takes the focus"
    );
}

pub(super) fn should_show_the_commands_unavailable_sign_again_after_a_notice() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_commands_unavailable();
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    assert!(
        !overlay.first_shows_commands_unavailable(),
        "the notice shows alone"
    );
    // When
    pump_run_loop(SHORT_NOTICE.as_secs_f64());
    // Then
    assert!(
        overlay.is_pill_visible(),
        "the sign is back after the notice"
    );
    assert!(overlay.first_shows_commands_unavailable());
    assert_eq!(overlay.first_label_value().as_deref(), Some("CMD"));
    assert_eq!(overlay.first_text_window_value(), None);
}

pub(super) fn should_hide_the_pill_when_voice_commands_work_again() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_commands_unavailable();
    pump_run_loop(DRAWN_SECS);
    assert!(overlay.is_pill_visible(), "the sign shows alone");
    // When
    overlay.hide_commands_unavailable();
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(!overlay.is_pill_visible(), "the pill hid with the sign");
}
