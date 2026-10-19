use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::core::domain::FailureNotice;

use super::main_queue::dispatch_to_main_after;
use super::pill_content::{glass_tint, state_sign, PillKind};
use super::pill_states::apply_hidden_state;
use super::pill_view::PillLook;
use super::window_set::WindowSet;
use super::OverlayState;

/// When a shown notice hides: `duration_ms` after it is drawn, unless a later signal moved the
/// pulse generation first.
#[derive(Clone)]
pub(super) struct NoticeTimer {
    pub(super) duration_ms: i64,
    pub(super) pulse_generation: Arc<AtomicU64>,
    pub(super) state: Arc<Mutex<OverlayState>>,
    pub(super) commands_unavailable: Arc<AtomicBool>,
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
        let commands_unavailable = timer.commands_unavailable.load(Ordering::SeqCst);
        apply_hidden_state(windows.lock().unwrap().clone(), commands_unavailable);
    });
}

/// Amber glass with the warning triangle and the failing part's short name, steady, with no
/// text window; the message goes to the log.
fn apply_failure_notice(windows: &[Arc<WindowSet>], notice: &FailureNotice) {
    let state = OverlayState::FailureNotice(*notice);
    let look = PillLook {
        kind: PillKind::Sign,
        sign: state_sign(state),
        breathes: false,
    };
    log::warn!("overlay: failure notice: {}", notice.message());
    for ws in windows {
        ws.draw(look, glass_tint(state), false, 0);
        ws.transcript.hide();
    }
}
