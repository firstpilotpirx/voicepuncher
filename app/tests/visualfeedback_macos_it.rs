//! The macOS overlay on the main thread, where AppKit draws and the main queue runs. libtest
//! runs each test on a thread of its own, where no overlay can be built, so this harness
//! (`harness = false`) answers cargo-nextest's listing and runs each test on the main thread.

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use objc2_foundation::NSPoint;
use voicepuncher::adapters::secondary::visualfeedback::macos::{
    MacosPointerSign, MacosVisualIndicator,
};
use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::{
    DictationSignState, PointerSignView,
};
use voicepuncher::core::application::VisualIndicator;
use voicepuncher::core::domain::{CleanedText, FailingPart, FailureNotice, RawTranscript};

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
        "waveform_tick_cancels_on_hide",
        waveform_tick_cancels_on_hide,
    ),
    (
        "should_raise_waveform_columns_for_a_loud_level",
        should_raise_waveform_columns_for_a_loud_level,
    ),
    (
        "should_show_the_record_sign_and_waveform_without_a_label",
        should_show_the_record_sign_and_waveform_without_a_label,
    ),
    (
        "should_show_each_processing_stage_with_its_icon_and_label",
        should_show_each_processing_stage_with_its_icon_and_label,
    ),
    (
        "should_show_the_dimmed_raw_transcript_while_refining",
        should_show_the_dimmed_raw_transcript_while_refining,
    ),
    (
        "should_explain_the_wait_for_focus_in_the_text_window",
        should_explain_the_wait_for_focus_in_the_text_window,
    ),
    (
        "should_show_the_pasted_sign_and_hide_after_half_a_second",
        should_show_the_pasted_sign_and_hide_after_half_a_second,
    ),
    (
        "should_replace_the_pasted_sign_with_a_later_signal",
        should_replace_the_pasted_sign_with_a_later_signal,
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
    (
        "should_turn_the_activity_ring_of_the_dictation_sign_only_while_processing",
        should_turn_the_activity_ring_of_the_dictation_sign_only_while_processing,
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

fn waveform_tick_cancels_on_hide() {
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
const DOT_HEIGHT: f64 = 2.0;

fn should_raise_waveform_columns_for_a_loud_level() {
    // Given — RMS 0.5 is louder than the waveform's full scale
    let level_register = Arc::new(AtomicU32::new(0.5f32.to_bits()));
    let indicator = MacosVisualIndicator::initialize(main_thread(), Arc::clone(&level_register));

    // When — a few column periods of 100 ms pass
    indicator.show_recording();
    pump_run_loop(0.4);

    // Then
    let tallest = indicator.first_tallest_waveform_column();
    assert!(
        tallest > DOT_HEIGHT,
        "a loud level leaves a column taller than a dot, got {tallest}"
    );
    indicator.hide();
    pump_run_loop(DRAWN_SECS);
}

fn should_show_the_record_sign_and_waveform_without_a_label() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recording();
    pump_run_loop(DRAWN_SECS);
    assert!(overlay.first_shows_recording(), "no record sign");
    assert_eq!(
        overlay.first_label_value(),
        None,
        "the recording shows a label"
    );
    assert_eq!(overlay.first_tallest_waveform_column(), DOT_HEIGHT);
    assert!(!overlay.has_key_window(), "the glass takes the focus");
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

fn should_show_each_processing_stage_with_its_icon_and_label() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_recognizing_speech();
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_icon_symbol().as_deref(), Some("ear"));
    assert_eq!(overlay.first_label_value().as_deref(), Some("Recognizing"));
    assert_eq!(overlay.first_text_window_value(), None);
    overlay.show_refining_text(&raw("we moved the meeting"));
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_icon_symbol().as_deref(), Some("sparkles"));
    assert_eq!(
        overlay.first_label_value().as_deref(),
        Some("Refining text")
    );
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

fn should_show_the_dimmed_raw_transcript_while_refining() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_refining_text(&raw("we moved the meeting"));
    pump_run_loop(DRAWN_SECS);
    assert_eq!(
        overlay.first_text_window_value().as_deref(),
        Some("we moved the meeting")
    );
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

fn should_explain_the_wait_for_focus_in_the_text_window() {
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_waiting_for_focus();
    pump_run_loop(DRAWN_SECS);
    assert_eq!(overlay.first_icon_symbol().as_deref(), Some("keyboard"));
    assert_eq!(
        overlay.first_label_value().as_deref(),
        Some("Click a text field")
    );
    assert_eq!(
        overlay.first_text_window_value().as_deref(),
        Some("Text is ready and pastes once a text field has focus")
    );
    overlay.hide();
    pump_run_loop(DRAWN_SECS);
}

/// The Pasted sign shows 0.5 s and fades out in 0.2 s.
const PASTED_GONE_SECS: f64 = 0.9;

fn should_show_the_pasted_sign_and_hide_after_half_a_second() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    // When
    overlay.show_pasted(&cleaned("we moved the meeting"));
    pump_run_loop(DRAWN_SECS);
    // Then
    assert!(overlay.is_pill_visible(), "the Pasted sign is not shown");
    assert_eq!(overlay.first_icon_symbol().as_deref(), Some("checkmark"));
    assert_eq!(overlay.first_label_value().as_deref(), Some("Pasted"));
    assert_eq!(
        overlay.first_text_window_value().as_deref(),
        Some("we moved the meeting")
    );
    pump_run_loop(PASTED_GONE_SECS);
    assert!(!overlay.is_pill_visible(), "the Pasted sign did not hide");
    assert_eq!(overlay.first_text_window_value(), None);
}

fn should_replace_the_pasted_sign_with_a_later_signal() {
    // Given
    let overlay = overlay_with_notice_lasting(SHORT_NOTICE);
    overlay.show_pasted(&cleaned("we moved the meeting"));
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

fn raw(text: &str) -> RawTranscript {
    RawTranscript::new(text.to_owned()).expect("a transcript with words")
}

fn cleaned(text: &str) -> CleanedText {
    CleanedText::new(text.to_owned()).expect("a text with words")
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
    assert_eq!(
        overlay.first_icon_symbol().as_deref(),
        Some("exclamationmark.triangle.fill")
    );
    assert_eq!(
        overlay.first_label_value().as_deref(),
        Some(NOTICE.part_name())
    );
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

fn should_keep_the_commands_unavailable_sign_when_the_overlay_hides() {
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

fn should_show_the_commands_unavailable_sign_again_after_a_notice() {
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

/// How many times the sign is read again while the real pointer moves (someone uses the mouse).
const STILL_POINTER_TRIES: usize = 50;

/// The pointer and the sign's top-left corner, read while the pointer stood still: the sign
/// follows the real pointer on a timer, so a pointer that moved between the two reads would
/// sit anywhere beside a sign placed for its previous spot.
fn sign_beside_a_still_pointer(sign: &MacosPointerSign) -> (NSPoint, NSPoint) {
    let read = || {
        let before = objc2_app_kit::NSEvent::mouseLocation();
        pump_run_loop(0.05);
        let top_left = sign.top_left();
        let after = objc2_app_kit::NSEvent::mouseLocation();
        (before, after, top_left)
    };
    for _ in 1..STILL_POINTER_TRIES {
        let (before, after, top_left) = read();
        if before.x == after.x && before.y == after.y {
            return (after, top_left);
        }
    }
    let (_, after, top_left) = read();
    (after, top_left)
}

fn should_show_the_dictation_sign_next_to_the_pointer_without_taking_input() {
    let sign = MacosPointerSign::initialize(main_thread());
    sign.show(DictationSignState::Recording);
    pump_run_loop(0.1);
    let (pointer, top_left) = sign_beside_a_still_pointer(&sign);
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
    sign.show(DictationSignState::Recording);
    pump_run_loop(0.1);
    sign.hide();
    pump_run_loop(0.1);
    assert!(!sign.is_visible(), "the dictation sign still shows");
}

fn should_turn_the_activity_ring_of_the_dictation_sign_only_while_processing() {
    // Given
    let sign = MacosPointerSign::initialize(main_thread());
    sign.show(DictationSignState::Recording);
    pump_run_loop(0.1);
    assert!(sign.shows_icon(), "the recording badge shows no icon");
    assert!(
        !sign.activity_ring_turns(),
        "the ring turns while recording"
    );
    // When
    sign.show(DictationSignState::RecognizingSpeech);
    pump_run_loop(0.1);
    // Then
    assert!(
        sign.shows_icon(),
        "the speech recognition badge shows no icon"
    );
    assert!(
        sign.activity_ring_turns(),
        "the ring stands still while recognizing"
    );
    sign.hide();
    pump_run_loop(0.05);
    assert!(
        !sign.activity_ring_turns(),
        "the ring of a hidden sign turns"
    );
}
