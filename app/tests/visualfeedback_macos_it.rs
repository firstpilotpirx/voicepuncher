//! The macOS overlay on the main thread, where AppKit draws and the main queue runs. libtest
//! runs each test on a thread of its own, where no overlay can be built, so this harness
//! (`harness = false`) answers cargo-nextest's listing and runs each test on the main thread.

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use voicepuncher::adapters::secondary::visualfeedback::macos::{
    MacosPointerSign, MacosVisualIndicator,
};
use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::PointerSignView;
use voicepuncher::core::application::VisualIndicator;
use voicepuncher::core::domain::{FailingPart, FailureNotice};

type Test = (&'static str, fn());

const TESTS: &[Test] = &[
    (
        "should_implement_visual_indicator_trait",
        should_implement_visual_indicator_trait,
    ),
    (
        "should_drop_window_set_without_panic",
        should_drop_window_set_without_panic,
    ),
    (
        "should_preserve_recording_state_after_manual_rebuild",
        should_preserve_recording_state_after_manual_rebuild,
    ),
    (
        "should_skip_rebuild_when_topology_identical",
        should_skip_rebuild_when_topology_identical,
    ),
    (
        "equalizer_tick_cancels_on_hide",
        equalizer_tick_cancels_on_hide,
    ),
    (
        "level_propagation_to_eq_label",
        level_propagation_to_eq_label,
    ),
    (
        "show_recording_survives_manual_rebuild",
        show_recording_survives_manual_rebuild,
    ),
    (
        "show_recording_rebuilds_unconditionally",
        show_recording_rebuilds_unconditionally,
    ),
    (
        "show_recognizing_speech_rebuilds_unconditionally",
        show_recognizing_speech_rebuilds_unconditionally,
    ),
    (
        "should_show_the_notice_sign_and_message_without_activating",
        should_show_the_notice_sign_and_message_without_activating,
    ),
    (
        "should_hide_the_notice_after_its_time",
        should_hide_the_notice_after_its_time,
    ),
    (
        "should_restart_the_time_of_a_notice_shown_again",
        should_restart_the_time_of_a_notice_shown_again,
    ),
    (
        "should_keep_a_recording_started_after_a_notice",
        should_keep_a_recording_started_after_a_notice,
    ),
    (
        "should_keep_the_commands_unavailable_sign_when_the_overlay_hides",
        should_keep_the_commands_unavailable_sign_when_the_overlay_hides,
    ),
    (
        "should_show_the_commands_unavailable_sign_again_after_a_notice",
        should_show_the_commands_unavailable_sign_again_after_a_notice,
    ),
    (
        "should_hide_the_pill_when_voice_commands_work_again",
        should_hide_the_pill_when_voice_commands_work_again,
    ),
    (
        "should_show_the_dictation_sign_next_to_the_pointer_without_taking_input",
        should_show_the_dictation_sign_next_to_the_pointer_without_taking_input,
    ),
    (
        "should_hide_the_dictation_sign",
        should_hide_the_dictation_sign,
    ),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--list", "--format", "terse", "--ignored"] => {}
        ["--list", "--format", "terse"] => {
            for (name, _) in TESTS {
                println!("{name}: test");
            }
        }
        ["--exact", name, ..] => {
            let (_, test) = TESTS
                .iter()
                .find(|(listed, _)| listed == name)
                .unwrap_or_else(|| panic!("no test named {name}"));
            test();
        }
        _ => TESTS.iter().for_each(|(_, test)| test()),
    }
}

fn main_thread() -> MainThreadMarker {
    MainThreadMarker::new().expect("the harness runs every test on the main thread")
}

fn implements_visual_indicator<T: VisualIndicator>() {}

fn pump_run_loop(secs: f64) {
    use objc2_foundation::{NSDate, NSRunLoop};
    let run_loop = NSRunLoop::currentRunLoop();
    let deadline = NSDate::dateWithTimeIntervalSinceNow(secs);
    run_loop.runUntilDate(&deadline);
}

fn should_implement_visual_indicator_trait() {
    implements_visual_indicator::<MacosVisualIndicator>();
}

fn should_drop_window_set_without_panic() {
    let mtm = main_thread();
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    drop(indicator);
}

fn should_preserve_recording_state_after_manual_rebuild() {
    let mtm = main_thread();
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    indicator.show_recording();
    indicator.rebuild_window_set(mtm);
}

fn should_skip_rebuild_when_topology_identical() {
    let mtm = main_thread();
    let indicator = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    indicator.rebuild_window_set(mtm);
    indicator.rebuild_window_set(mtm);
}

fn equalizer_tick_cancels_on_hide() {
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
}

fn level_propagation_to_eq_label() {
    let mtm = main_thread();

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

fn show_recording_survives_manual_rebuild() {
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

fn show_recording_rebuilds_unconditionally() {
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

fn show_recognizing_speech_rebuilds_unconditionally() {
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

/// A notice that lasts long enough for the tests to look at it, and short enough not to wait.
const SHORT_NOTICE: Duration = Duration::from_millis(600);
/// Long enough for the main queue to draw what was asked.
const DRAWN_SECS: f64 = 0.2;

const NOTICE: FailureNotice = FailureNotice::DictationLost(FailingPart::SpeechRecognition);

fn overlay_with_notice_lasting(duration: Duration) -> MacosVisualIndicator {
    MacosVisualIndicator::initialize_with_notice_duration(
        main_thread(),
        Arc::new(AtomicU32::new(0)),
        duration,
    )
}

fn should_show_the_notice_sign_and_message_without_activating() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    // When
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(overlay.is_pill_visible(), "the pill shows the notice");
    assert_eq!(overlay.first_sign_value(), "⚠");
    assert_eq!(overlay.first_eq_label_value(), NOTICE.part_name());
    assert_eq!(overlay.first_text_window_value(), Some(NOTICE.message()));
    assert!(
        !overlay.has_key_window(),
        "no overlay window takes the focus"
    );
    assert!(!NSApplication::sharedApplication(main_thread()).isActive());
}

fn should_hide_the_notice_after_its_time() {
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

fn should_restart_the_time_of_a_notice_shown_again() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    let most_of_the_time = SHORT_NOTICE.as_secs_f64() * 2.0 / 3.0;
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(most_of_the_time);
    // When
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(most_of_the_time);
    // Then
    assert!(
        overlay.is_pill_visible(),
        "the first notice's time no longer counts"
    );
    assert_eq!(overlay.first_text_window_value(), Some(NOTICE.message()));
    pump_run_loop(most_of_the_time);
    assert!(
        !overlay.is_pill_visible(),
        "the notice shown again hid after its own time"
    );
}

fn should_keep_a_recording_started_after_a_notice() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_sign_value(), "⚠", "the notice is shown");
    // When
    overlay.show_recording();
    pump_run_loop(SHORT_NOTICE.as_secs_f64() + DRAWN_SECS);
    // Then
    assert!(
        overlay.is_pill_visible(),
        "the notice's timer left the recording shown"
    );
    assert_eq!(overlay.first_sign_value(), "◉ REC");
    assert_eq!(overlay.first_text_window_value(), None);
}

fn should_keep_the_commands_unavailable_sign_when_the_overlay_hides() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recording();
    overlay.show_commands_unavailable();
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_sign_value(), "◉ REC ⊘ CMD");
    // When
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(overlay.is_pill_visible(), "the sign stays after the hide");
    assert_eq!(overlay.first_sign_value(), "⊘ CMD");
    assert!(
        !overlay.has_key_window(),
        "no overlay window takes the focus"
    );
}

fn should_show_the_commands_unavailable_sign_again_after_a_notice() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_commands_unavailable();
    overlay.show_failure_notice(&NOTICE);
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_sign_value(), "⚠", "the notice shows alone");
    // When
    pump_run_loop(SHORT_NOTICE.as_secs_f64());
    // Then
    assert!(
        overlay.is_pill_visible(),
        "the sign is back after the notice"
    );
    assert_eq!(overlay.first_sign_value(), "⊘ CMD");
    assert_eq!(overlay.first_text_window_value(), None);
}

fn should_hide_the_pill_when_voice_commands_work_again() {
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

fn should_show_the_dictation_sign_next_to_the_pointer_without_taking_input() {
    let sign = MacosPointerSign::initialize(main_thread());
    sign.show();
    pump_run_loop(0.1);
    let pointer = objc2_app_kit::NSEvent::mouseLocation();
    let top_left = sign.top_left();
    assert!(sign.is_visible(), "the dictation sign is not shown");
    assert!(
        sign.ignores_mouse_events(),
        "the dictation sign takes clicks"
    );
    assert!(
        !sign.can_become_key(),
        "the dictation sign can take the keyboard focus"
    );
    assert!(
        top_left.x > pointer.x && top_left.y < pointer.y,
        "the dictation sign is not right of and below the pointer: sign {top_left:?}, pointer {pointer:?}"
    );
    sign.hide();
    pump_run_loop(0.05);
}

fn should_hide_the_dictation_sign() {
    let sign = MacosPointerSign::initialize(main_thread());
    sign.show();
    pump_run_loop(0.1);
    sign.hide();
    pump_run_loop(0.1);
    assert!(!sign.is_visible(), "the dictation sign still shows");
}
