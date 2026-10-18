use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use objc2_app_kit::{NSColor, NSWindowCollectionBehavior};
use objc2_foundation::NSString;

use crate::core::domain::FailureNotice;

use super::window_set::WindowSet;
use super::{apply_hidden_state, dispatch_to_main_after, OverlayState};

const NOTICE_SIGN: &str = "⚠";

/// What the pill shows for `notice`: the warning sign, and the failing part's name where the
/// level shows.
fn notice_sign(notice: &FailureNotice) -> (&'static str, &'static str) {
    (NOTICE_SIGN, notice.part_name())
}

/// When a shown notice hides: `duration_ms` after it is drawn, unless a later signal moved the
/// pulse generation first.
#[derive(Clone)]
pub(super) struct NoticeTimer {
    pub(super) duration_ms: i64,
    pub(super) pulse_generation: Arc<AtomicU64>,
    pub(super) state: Arc<Mutex<OverlayState>>,
}

/// Draws `notice` on every screen, on the main queue, and hides it once its time is over.
pub(super) fn show_notice(
    windows: &Arc<Mutex<Vec<Arc<WindowSet>>>>,
    notice: &FailureNotice,
    generation: u64,
    timer: NoticeTimer,
) {
    apply_failure_notice(&windows.lock().unwrap(), notice);
    let windows = Arc::clone(windows);
    dispatch_to_main_after(timer.duration_ms, move || {
        // WHY: under the state lock, so a signal shown meanwhile keeps its state
        let mut state = timer.state.lock().unwrap();
        if timer.pulse_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        *state = OverlayState::Hidden;
        drop(state);
        apply_hidden_state(windows.lock().unwrap().clone());
    });
}

/// The notice sign in the pill, steady, with no spinner; the message in the window under it.
fn apply_failure_notice(windows: &[Arc<WindowSet>], notice: &FailureNotice) {
    let (icon, name) = notice_sign(notice);
    let (icon, name) = (NSString::from_str(icon), NSString::from_str(name));
    let message = notice.message();
    for ws in windows {
        ws.label.setStringValue(&icon);
        ws.label.setTextColor(Some(&NSColor::systemYellowColor()));
        ws.label.setAlphaValue(1.0);
        ws.label.setHidden(false);
        ws.eq_label.setStringValue(&name);
        ws.eq_label.setAlphaValue(1.0);
        ws.eq_label.setHidden(false);
        ws.spinner.setHidden(true);
        unsafe { ws.spinner.stopAnimation(None) };
        ws.transcript.show(Some(&message));
        ws.window.orderFrontRegardless();
        ws.window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::FailingPart;

    #[test]
    fn should_sign_a_notice_with_the_warning_sign_and_the_failing_part() {
        let notice = FailureNotice::DictationLost(FailingPart::SpeechRecognition);
        assert_eq!(notice_sign(&notice), ("⚠", "STT"));
    }

    #[test]
    fn should_sign_a_notice_without_a_part_as_failed() {
        let notice = FailureNotice::NotPasted(None);
        assert_eq!(notice_sign(&notice), ("⚠", "FAILED"));
    }
}
