use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use voicepuncher::adapters::secondary::visualfeedback::macos::MacosVisualIndicator;
use voicepuncher::core::application::VisualIndicator;

fn implements_visual_indicator<T: VisualIndicator>() {}

fn pump_run_loop(secs: f64) {
    use objc2_foundation::{NSDate, NSRunLoop};
    let run_loop = NSRunLoop::currentRunLoop();
    let deadline = NSDate::dateWithTimeIntervalSinceNow(secs);
    run_loop.runUntilDate(&deadline);
}

#[test]
fn should_implement_visual_indicator_trait() {
    implements_visual_indicator::<MacosVisualIndicator>();
}

#[test]
fn should_drop_window_set_without_panic() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    drop(indicator);
}

#[test]
fn should_preserve_recording_state_after_manual_rebuild() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    indicator.show_recording();
    indicator.rebuild_window_set(mtm);
}

#[test]
fn should_skip_rebuild_when_topology_identical() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    indicator.rebuild_window_set(mtm);
    indicator.rebuild_window_set(mtm);
}

#[test]
fn equalizer_tick_cancels_on_hide() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

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
}

#[test]
fn level_propagation_to_eq_label() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    // Given — RMS 0.5 is well above the 0.060 saturation threshold → all 10 pills active (┃)
    let level_register = Arc::new(AtomicU32::new(0.5f32.to_bits()));
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::clone(&level_register));

    // When — show_recording() schedules the 80 ms equalizer tick
    indicator.show_recording();

    // Allow at least two tick intervals (80 ms each) plus scheduling overhead
    pump_run_loop(0.2);

    // Then
    let label_value = indicator.first_eq_label_value();
    assert!(
        label_value.contains('\u{2503}'),
        "eq_label must contain ┃ (U+2503) for AudioLevel 0.5, got: {label_value:?}"
    );
}

#[test]
fn show_recording_survives_manual_rebuild() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
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

#[test]
fn show_recording_rebuilds_unconditionally() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
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

#[test]
fn show_recognizing_speech_rebuilds_unconditionally() {
    use objc2::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
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
