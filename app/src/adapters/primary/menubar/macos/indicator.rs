//! The status item's link to the app: a decorator over the overlay and an observer of the
//! runtime, both writing one state cell, from which the icon and the menu are drawn.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::core::application::live_transcript_view::LiveTranscriptView;
use crate::core::application::VisualIndicator;
use crate::core::domain::{FailureNotice, VoiceCommand};
use crate::runtime::{RuntimeObserver, RuntimeState};

use super::mapping::{title_line, IconState, OverlaySignal, StatusItemState};

/// Where the status item is drawn: the menu bar in the app, a double in tests.
pub trait StatusItemSurface: Send + Sync {
    /// Draws `icon`; called from any thread, in the order the icon changes.
    fn show_icon(&self, icon: IconState);
}

/// The two lines of text at the top of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuModel {
    pub title_line: String,
    pub status_line: String,
}

/// What the status item was told, shared by the overlay decorator and the runtime observer.
#[derive(Clone)]
pub struct StatusItemCell {
    shared: Arc<Shared>,
}

struct Shared {
    state: Mutex<(StatusItemState, IconState)>,
    surface: Box<dyn StatusItemSurface>,
    /// Counts failure notices, so only the latest one's lifetime takes the warning down.
    notices: AtomicU64,
    notice_lifetime: Duration,
}

impl StatusItemCell {
    /// Draws the waiting icon at once; a failure notice shows the warning for
    /// `notice_lifetime`, as long as the overlay shows it.
    pub fn new(surface: Box<dyn StatusItemSurface>, notice_lifetime: Duration) -> Self {
        let state = StatusItemState::default();
        let icon = state.icon();
        surface.show_icon(icon);
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new((state, icon)),
                surface,
                notices: AtomicU64::new(0),
                notice_lifetime,
            }),
        }
    }

    /// The menu as it reads at this moment.
    pub fn menu(&self) -> MenuModel {
        MenuModel { title_line: title_line(), status_line: self.lock().0.status_line() }
    }

    /// `overlay`, also telling the status item each thing it shows.
    pub fn indicator<I: VisualIndicator>(&self, overlay: I) -> StatusItemIndicator<I> {
        StatusItemIndicator { overlay, cell: self.clone() }
    }

    /// Tells the status item each state of the runtime.
    pub fn observer(&self) -> StatusItemObserver {
        StatusItemObserver(self.clone())
    }
}

impl StatusItemCell {
    fn lock(&self) -> MutexGuard<'_, (StatusItemState, IconState)> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Changes the state and draws the icon when it changed; under the lock, so the icons
    /// are drawn in the order the state changed.
    fn change(&self, change: impl FnOnce(&mut StatusItemState)) {
        let mut guard = self.lock();
        let (state, shown) = &mut *guard;
        change(state);
        let icon = state.icon();
        if icon != *shown {
            *shown = icon;
            self.shared.surface.show_icon(icon);
        }
    }

    fn apply(&self, signal: OverlaySignal) {
        self.change(|state| state.apply(signal));
    }

    /// Shows the warning, and takes it down once the notice's lifetime ends unless a newer
    /// notice came meanwhile.
    fn failure_notice(&self) {
        let notice = self.shared.notices.fetch_add(1, Ordering::SeqCst) + 1;
        self.apply(OverlaySignal::FailureNotice);
        let cell = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(cell.shared.notice_lifetime);
            if cell.shared.notices.load(Ordering::SeqCst) == notice {
                cell.apply(OverlaySignal::FailureNoticeExpired);
            }
        });
    }
}

/// The overlay, with the status item told what it shows.
pub struct StatusItemIndicator<I> {
    overlay: I,
    cell: StatusItemCell,
}

impl<I: VisualIndicator> VisualIndicator for StatusItemIndicator<I> {
    fn show_recording(&self) {
        self.overlay.show_recording();
        self.cell.apply(OverlaySignal::Recording);
    }

    fn show_recognizing_speech(&self) {
        self.overlay.show_recognizing_speech();
        self.cell.apply(OverlaySignal::RecognizingSpeech);
    }

    fn show_waiting_for_focus(&self) {
        self.overlay.show_waiting_for_focus();
        self.cell.apply(OverlaySignal::WaitingForFocus);
    }

    fn show_pasted(&self) {
        self.overlay.show_pasted();
        self.cell.apply(OverlaySignal::Pasted);
    }

    fn hide(&self) {
        self.overlay.hide();
        self.cell.apply(OverlaySignal::Hidden);
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.overlay.show_failure_notice(notice);
        self.cell.failure_notice();
    }

    fn show_command_sign(&self, command: VoiceCommand) {
        self.overlay.show_command_sign(command);
        self.cell.apply(OverlaySignal::CommandSign);
    }

    fn show_commands_unavailable(&self) {
        self.overlay.show_commands_unavailable();
        self.cell.apply(OverlaySignal::CommandsUnavailable);
    }

    fn hide_commands_unavailable(&self) {
        self.overlay.hide_commands_unavailable();
        self.cell.apply(OverlaySignal::CommandsAvailable);
    }

    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        self.overlay.live_transcript_view()
    }
}

/// Tells the status item each runtime state.
pub struct StatusItemObserver(StatusItemCell);

impl RuntimeObserver for StatusItemObserver {
    fn state_changed(&self, state: &RuntimeState) {
        self.0.change(|shown| shown.runtime_changed(state.clone()));
    }
}
