//! The dictation sign next to the mouse pointer: a badge that mirrors the overlay's signals.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::self_hide::SelfHideTimer;
use crate::adapters::locking::lock;
use crate::core::application::live_transcript_view::LiveTranscriptView;
use crate::core::application::{OnDrawn, VisualIndicator};
use crate::core::domain::{FailureNotice, VoiceCommand};

/// What the dictation sign shows: the stage of the dictation the overlay shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationSignState {
    Recording,
    RecognizingSpeech,
    WaitingForFocus,
    Pasted,
    Failure,
}

impl DictationSignState {
    /// Whether an activity ring turns around the icon: while a local service works on the text.
    pub fn turns_activity_ring(self) -> bool {
        matches!(self, Self::RecognizingSpeech)
    }

    /// Whether the icon breathes: while the dictation waits for something to finish.
    pub fn breathes(self) -> bool {
        matches!(self, Self::RecognizingSpeech | Self::WaitingForFocus)
    }
}

/// The window that shows the dictation sign next to the pointer.
pub trait PointerSignView: Send + Sync + 'static {
    fn show(&self, badge: DictationSignState);
    fn hide(&self);
}

/// How long the overlay shows the Pasted sign, and the Pasted badge with it, in milliseconds.
pub const PASTED_SIGN_MS: i64 = 500;
/// How long the overlay shows the Pasted sign, and the Pasted badge with it.
pub const PASTED_SIGN_LIFETIME: Duration = Duration::from_millis(PASTED_SIGN_MS.unsigned_abs());
/// How long the overlay shows a failure notice, and the failure badge with it.
pub const FAILURE_NOTICE_LIFETIME: Duration = Duration::from_secs(4);

/// How long the badges that end a dictation show before they hide themselves: as long as the
/// overlay shows the Pasted sign and a failure notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignLifetimes {
    pub pasted: Duration,
    pub failure: Duration,
}

impl Default for SignLifetimes {
    fn default() -> Self {
        Self { pasted: PASTED_SIGN_LIFETIME, failure: FAILURE_NOTICE_LIFETIME }
    }
}

/// The overlay, plus the dictation sign while a dictation runs.
///
/// Each overlay signal shows its badge, the Pasted and failure badges hide themselves after their
/// lifetime, hiding the overlay hides the sign, and a command sign or the commands-unavailable sign
/// leave it as it is.
pub struct PointerSignIndicator<V, S> {
    overlay: V,
    sign: Arc<S>,
    lifetimes: SignLifetimes,
    /// Bumped by every change of the sign; a self-hide of an older badge does nothing. Held
    /// while the sign changes, so a self-hide never lands after a later badge.
    generation: Arc<Mutex<u64>>,
    /// Hides the Pasted or failure badge after its lifetime; dropped with the indicator.
    timer: SelfHideTimer,
}

impl<V: VisualIndicator, S: PointerSignView> PointerSignIndicator<V, S> {
    pub fn new(overlay: V, sign: S) -> Self {
        Self::with_lifetimes(overlay, sign, SignLifetimes::default())
    }

    pub fn with_lifetimes(overlay: V, sign: S, lifetimes: SignLifetimes) -> Self {
        let sign = Arc::new(sign);
        let generation = Arc::new(Mutex::new(0));
        let timer = SelfHideTimer::hiding(Arc::clone(&sign), Arc::clone(&generation));
        Self { overlay, sign, lifetimes, generation, timer }
    }

    fn show_badge(&self, badge: DictationSignState) {
        let mut generation = lock(&self.generation);
        *generation += 1;
        self.sign.show(badge);
    }

    fn hide_badge(&self) {
        let mut generation = lock(&self.generation);
        *generation += 1;
        self.sign.hide();
    }

    /// Shows `badge` and hides it after `lifetime`, unless the sign changed meanwhile.
    fn show_badge_for(&self, badge: DictationSignState, lifetime: Duration) {
        let mut generation = lock(&self.generation);
        *generation += 1;
        self.sign.show(badge);
        self.timer.hide_after(lifetime, *generation);
    }
}

impl<V: VisualIndicator, S: PointerSignView> VisualIndicator for PointerSignIndicator<V, S> {
    fn show_recording(&self) {
        self.show_badge(DictationSignState::Recording);
        self.overlay.show_recording();
    }

    fn show_recording_then(&self, on_drawn: OnDrawn) {
        self.show_badge(DictationSignState::Recording);
        self.overlay.show_recording_then(on_drawn);
    }

    fn show_recognizing_speech(&self) {
        self.show_badge(DictationSignState::RecognizingSpeech);
        self.overlay.show_recognizing_speech();
    }

    fn show_waiting_for_focus(&self) {
        self.show_badge(DictationSignState::WaitingForFocus);
        self.overlay.show_waiting_for_focus();
    }

    fn hide(&self) {
        self.hide_badge();
        self.overlay.hide();
    }

    fn show_pasted(&self) {
        self.show_badge_for(DictationSignState::Pasted, self.lifetimes.pasted);
        self.overlay.show_pasted();
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.show_badge_for(DictationSignState::Failure, self.lifetimes.failure);
        self.overlay.show_failure_notice(notice);
    }

    fn show_command_sign(&self, command: VoiceCommand) {
        self.overlay.show_command_sign(command);
    }

    fn show_commands_unavailable(&self) {
        self.overlay.show_commands_unavailable();
    }

    fn hide_commands_unavailable(&self) {
        self.overlay.hide_commands_unavailable();
    }

    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        self.overlay.live_transcript_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::domain::FailingPart;

    /// Short enough for a quick test, long enough to tell "not yet" from "gone".
    const SHORT_LIFETIME: Duration = Duration::from_millis(30);
    /// Long after every short lifetime ran out.
    const AFTER_LIFETIME: Duration = Duration::from_millis(300);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SignCall {
        Show(DictationSignState),
        Hide,
    }

    #[derive(Clone, Default)]
    pub(crate) struct Sign(Arc<Mutex<Vec<SignCall>>>);

    impl Sign {
        fn calls(&self) -> Vec<SignCall> {
            self.0.lock().unwrap().clone()
        }
    }

    impl PointerSignView for Sign {
        fn show(&self, badge: DictationSignState) {
            self.0.lock().unwrap().push(SignCall::Show(badge));
        }

        fn hide(&self) {
            self.0.lock().unwrap().push(SignCall::Hide);
        }
    }

    fn quiet_overlay() -> MockVisualIndicator {
        crate::core::application::prelude::accepting_overlay()
    }

    fn indicator(sign: &Sign) -> PointerSignIndicator<MockVisualIndicator, Sign> {
        let lifetimes = SignLifetimes { pasted: SHORT_LIFETIME, failure: SHORT_LIFETIME };
        PointerSignIndicator::with_lifetimes(quiet_overlay(), sign.clone(), lifetimes)
    }

    #[test]
    fn should_show_the_badge_of_each_stage_of_a_dictation() {
        // Given
        let sign = Sign::default();
        let indicator = indicator(&sign);
        // When
        indicator.show_recording();
        indicator.show_recognizing_speech();
        indicator.show_waiting_for_focus();
        indicator.show_pasted();
        // Then
        let expected = [
            DictationSignState::Recording,
            DictationSignState::RecognizingSpeech,
            DictationSignState::WaitingForFocus,
            DictationSignState::Pasted,
        ]
        .map(SignCall::Show);
        assert_eq!(sign.calls(), expected);
    }

    #[test]
    fn should_show_the_failure_badge_for_a_failure_notice() {
        let sign = Sign::default();
        let notice = FailureNotice::DictationLost(FailingPart::SpeechRecognition);
        indicator(&sign).show_failure_notice(&notice);
        assert_eq!(sign.calls(), [SignCall::Show(DictationSignState::Failure)]);
    }

    #[test]
    fn should_hide_the_pasted_badge_after_its_lifetime() {
        let sign = Sign::default();
        let indicator = indicator(&sign);
        indicator.show_pasted();
        std::thread::sleep(AFTER_LIFETIME);
        let expected = [SignCall::Show(DictationSignState::Pasted), SignCall::Hide];
        assert_eq!(sign.calls(), expected);
    }

    #[test]
    fn should_hide_the_failure_badge_after_its_lifetime() {
        let sign = Sign::default();
        let indicator = indicator(&sign);
        indicator.show_failure_notice(&FailureNotice::RecordingNotStarted);
        std::thread::sleep(AFTER_LIFETIME);
        let expected = [SignCall::Show(DictationSignState::Failure), SignCall::Hide];
        assert_eq!(sign.calls(), expected);
    }

    #[test]
    fn should_keep_a_later_badge_when_an_earlier_one_runs_out() {
        // Given
        let sign = Sign::default();
        let indicator = indicator(&sign);
        indicator.show_pasted();
        // When
        indicator.show_recording();
        std::thread::sleep(AFTER_LIFETIME);
        // Then
        let expected =
            [DictationSignState::Pasted, DictationSignState::Recording].map(SignCall::Show);
        assert_eq!(sign.calls(), expected);
    }

    #[test]
    fn should_hide_the_badge_with_the_overlay() {
        let sign = Sign::default();
        let indicator = indicator(&sign);
        indicator.show_recording();
        indicator.hide();
        let expected = [
            SignCall::Show(DictationSignState::Recording),
            SignCall::Hide,
        ];
        assert_eq!(sign.calls(), expected);
    }

    #[test]
    fn should_leave_the_badge_as_it_was_on_a_command_sign() {
        let sign = Sign::default();
        let indicator = indicator(&sign);
        indicator.show_recording();
        indicator.show_command_sign(VoiceCommand::PressEnter);
        assert_eq!(
            sign.calls(),
            [SignCall::Show(DictationSignState::Recording)]
        );
    }

    #[test]
    fn should_turn_the_activity_ring_only_while_a_service_works_on_the_text() {
        let turning = [DictationSignState::RecognizingSpeech];
        for badge in all_badges() {
            assert_eq!(
                badge.turns_activity_ring(),
                turning.contains(&badge),
                "{badge:?}"
            );
        }
    }

    #[test]
    fn should_breathe_only_while_the_dictation_waits() {
        let breathing = [
            DictationSignState::RecognizingSpeech,
            DictationSignState::WaitingForFocus,
        ];
        for badge in all_badges() {
            assert_eq!(badge.breathes(), breathing.contains(&badge), "{badge:?}");
        }
    }

    fn all_badges() -> [DictationSignState; 5] {
        [
            DictationSignState::Recording,
            DictationSignState::RecognizingSpeech,
            DictationSignState::WaitingForFocus,
            DictationSignState::Pasted,
            DictationSignState::Failure,
        ]
    }

    #[test]
    fn should_pass_the_commands_unavailable_sign_to_the_overlay_and_leave_the_badge_alone() {
        // Given
        let mut overlay = MockVisualIndicator::new();
        overlay
            .expect_show_commands_unavailable()
            .times(1)
            .return_const(());
        overlay
            .expect_hide_commands_unavailable()
            .times(1)
            .return_const(());
        let sign = Sign::default();
        let indicator = PointerSignIndicator::new(overlay, sign.clone());
        // When
        indicator.show_commands_unavailable();
        indicator.hide_commands_unavailable();
        // Then
        assert!(sign.calls().is_empty());
    }
}
