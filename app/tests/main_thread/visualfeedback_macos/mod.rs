//! The macOS overlay on the main thread, where `AppKit` draws and the main queue runs. libtest
//! runs each test on a thread of its own, where no overlay can be built, so this harness
//! (`harness = false`) answers cargo-nextest's listing and runs each test on the main thread.

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use objc2_app_kit::NSApplication;
use objc2_foundation::NSPoint;
use voicepuncher::adapters::secondary::visualfeedback::macos::{
    GlassFinish, MacosPointerSign, MacosVisualIndicator,
};
use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::{
    DictationSignState, PointerSignView,
};
use voicepuncher::core::application::VisualIndicator;
use voicepuncher::core::domain::{FailingPart, FailureNotice};

use crate::{main_thread, Test};

mod recording;
use recording::{
    should_draw_clear_glass_without_a_top_line, should_hang_the_live_transcript_below_the_pill,
    should_keep_the_pill_size_from_the_recording_to_pasted,
    should_keep_the_recording_pill_when_the_screens_are_unchanged,
    should_pulse_the_record_dot_in_silence, should_raise_waveform_columns_for_a_loud_level,
    should_show_each_processing_stage_with_its_icon_and_label,
    should_show_the_record_sign_and_waveform_without_a_label,
    should_show_the_wait_for_focus_without_a_text_window, waveform_tick_cancels_on_hide,
};
mod pasted;
use pasted::{
    should_replace_the_pasted_sign_with_a_later_signal,
    should_show_the_pasted_sign_and_hide_after_half_a_second,
    should_take_the_text_window_away_when_the_recording_ends,
    show_recognizing_speech_rebuilds_unconditionally, show_recording_rebuilds_unconditionally,
    show_recording_survives_manual_rebuild, PASTED_GONE_SECS,
};
mod notices;
use notices::{
    overlay_with_notice_lasting, should_hide_the_notice_after_its_time,
    should_hide_the_pill_when_voice_commands_work_again,
    should_keep_a_recording_started_after_a_notice,
    should_keep_the_commands_unavailable_sign_when_the_overlay_hides,
    should_log_the_notice_message_as_a_warning, should_restart_the_time_of_a_notice_shown_again,
    should_show_the_commands_unavailable_sign_again_after_a_notice,
    should_show_the_notice_sign_without_a_text_window_or_activating, DRAWN_LIMIT, DRAWN_SECS,
    PUMP_STEP_SECS, SHORT_NOTICE,
};
mod pointer;
use pointer::{
    should_hide_the_dictation_sign,
    should_show_the_dictation_sign_next_to_the_pointer_without_taking_input,
    should_turn_the_activity_ring_of_the_dictation_sign_only_while_processing,
};

pub const TESTS: &[Test] = &[
    (
        "should_keep_the_recording_pill_when_the_screens_are_unchanged",
        should_keep_the_recording_pill_when_the_screens_are_unchanged,
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
        "should_draw_clear_glass_without_a_top_line",
        should_draw_clear_glass_without_a_top_line,
    ),
    (
        "should_hang_the_live_transcript_below_the_pill",
        should_hang_the_live_transcript_below_the_pill,
    ),
    (
        "should_pulse_the_record_dot_in_silence",
        should_pulse_the_record_dot_in_silence,
    ),
    (
        "should_keep_the_pill_size_from_the_recording_to_pasted",
        should_keep_the_pill_size_from_the_recording_to_pasted,
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
        "should_show_the_wait_for_focus_without_a_text_window",
        should_show_the_wait_for_focus_without_a_text_window,
    ),
    (
        "should_take_the_text_window_away_when_the_recording_ends",
        should_take_the_text_window_away_when_the_recording_ends,
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
        "should_show_the_notice_sign_without_a_text_window_or_activating",
        should_show_the_notice_sign_without_a_text_window_or_activating,
    ),
    (
        "should_log_the_notice_message_as_a_warning",
        should_log_the_notice_message_as_a_warning,
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

fn pump_run_loop(secs: f64) {
    use objc2_foundation::{NSDate, NSRunLoop};
    let run_loop = NSRunLoop::currentRunLoop();
    let deadline = NSDate::dateWithTimeIntervalSinceNow(secs);
    run_loop.runUntilDate(&deadline);
}

/// Runs the main run loop in short steps until `moment`.
fn pump_until_moment(moment: Instant) {
    while Instant::now() < moment {
        pump_run_loop(PUMP_STEP_SECS);
    }
}

/// Runs the main run loop in short steps until `condition` holds or `limit` passes.
fn pump_until(limit: Duration, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + limit;
    while !condition() && Instant::now() < deadline {
        pump_run_loop(PUMP_STEP_SECS);
    }
}
