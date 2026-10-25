use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use objc2::MainThreadMarker;

use crate::core::domain::FailureNotice;

use super::pill_content::{glass_tint, state_sign, PillKind};
use super::pill_states::apply_hidden_state;
use super::pill_view::PillLook;
use super::window_set::{SharedWindows, WindowSet};
use super::OverlayState;
use crate::adapters::locking::lock;
use crate::adapters::main_queue::on_main_after;

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
    (main, mtm): (&SharedWindows, MainThreadMarker),
    notice: FailureNotice,
    generation: u64,
    timer: NoticeTimer,
) {
    apply_failure_notice(&main.windows(mtm), notice);
    let main = main.clone();
    on_main_after(timer.duration_ms, move |mtm| {
        // WHY: under the state lock, so a signal shown meanwhile keeps its state
        let mut state = lock(&timer.state);
        if timer.pulse_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        *state = OverlayState::Hidden;
        drop(state);
        let commands_unavailable = timer.commands_unavailable.load(Ordering::SeqCst);
        apply_hidden_state(&main.windows(mtm), commands_unavailable);
    });
}

/// Amber glass with the warning triangle and the failing part's short name, steady, with no
/// text window; the message goes to the log.
fn apply_failure_notice(windows: &[Rc<WindowSet>], notice: FailureNotice) {
    let state = OverlayState::FailureNotice(notice);
    let look = PillLook { kind: PillKind::Sign, sign: state_sign(state), breathes: false };
    log::warn!("overlay: failure notice: {}", notice.message());
    for ws in windows {
        ws.draw(look, glass_tint(state), false, 0);
        ws.transcript.hide();
    }
}
