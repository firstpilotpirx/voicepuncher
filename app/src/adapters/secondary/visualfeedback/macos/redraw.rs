//! Rebuilding the overlay windows when the screens change.

use super::{
    apply_hidden_state, apply_live_transcript, apply_processing_state, apply_recording_state,
    apply_waiting_for_focus_state, lock, on_main, show_notice, Arc, LevelFeed, LiveTranscriptText,
    MainThreadMarker, NSRect, NSScreen, NoticeTimer, Ordering, OverlayState, ScreenTopology,
    SharedWindows, Turn, WindowSet, Windows, FALLBACK_SCREEN,
};
use super::{diff_screens, ScreenChange};
use std::sync::atomic::AtomicU64;

/// Rebuilds the windows for the screens of the moment and redraws the current state.
pub(super) fn screen_change_handler(redraw: Redraw) -> Arc<dyn Fn() + Send + Sync + 'static> {
    Arc::new(move || {
        let redraw = redraw.clone();
        on_main(move |mtm| redraw.rebuild_and_draw(mtm));
    })
}

/// How many times the windows were built, in this process.
pub(super) static WINDOW_BUILDS: AtomicU64 = AtomicU64::new(0);

pub(super) fn build_windows(mtm: MainThreadMarker) -> Windows {
    WINDOW_BUILDS.fetch_add(1, Ordering::SeqCst);
    let screens = NSScreen::screens(mtm);
    if screens.is_empty() {
        return vec![WindowSet::build(mtm, FALLBACK_SCREEN)];
    }
    (0..screens.count())
        .map(|i| WindowSet::build(mtm, screens.objectAtIndex(i).visibleFrame()))
        .collect()
}

pub(super) fn do_rebuild(mtm: MainThreadMarker, main: &SharedWindows) {
    if let Some(main) = main.get(mtm) {
        main.windows.replace(build_windows(mtm));
        main.topology.replace(collect_topology(mtm));
    }
}

/// Rebuilds the windows only when the screens changed since they were built.
pub(super) fn rebuild_if_screens_changed(mtm: MainThreadMarker, main: &SharedWindows) {
    let changed = main.get(mtm).is_some_and(|main| {
        diff_screens(&main.topology.borrow(), &collect_topology(mtm)) != ScreenChange::Identical
    });
    if changed {
        do_rebuild(mtm, main);
    }
}

pub(super) fn collect_topology(mtm: MainThreadMarker) -> ScreenTopology {
    let screens = NSScreen::screens(mtm);
    let frames: Vec<NSRect> = (0..screens.count())
        .map(|i| screens.objectAtIndex(i).visibleFrame())
        .collect();
    ScreenTopology { frames }
}

/// What draws a state of the overlay on the windows of the moment, with the
/// commands-unavailable sign while it is on.
#[derive(Clone)]
pub(super) struct Redraw {
    pub(crate) main: SharedWindows,
    pub(crate) feed: LevelFeed,
    pub(crate) live_transcript: LiveTranscriptText,
    pub(crate) timer: NoticeTimer,
}

impl Redraw {
    /// Rebuilds the windows for the screens of the moment and draws the current state.
    pub(crate) fn rebuild_and_draw(&self, mtm: MainThreadMarker) {
        let turn = Turn::next(&self.timer.pulse_generation);
        do_rebuild(mtm, &self.main);
        let current_state = lock(&self.timer.state).clone();
        self.draw_state(mtm, &current_state, turn);
    }

    /// Draws `state`, then builds the windows again and draws it once more when a pill it
    /// shows is not on the desktop the user is on.
    pub(crate) fn draw(&self, mtm: MainThreadMarker, state: &OverlayState, turn: Turn) {
        self.draw_state(mtm, state, turn);
        self.keep_on_the_current_desktop(mtm, state);
    }

    /// Brings a shown `state` onto the current desktop: fresh windows join every desktop, so
    /// windows the window server dropped from this one are built again.
    pub(crate) fn keep_on_the_current_desktop(&self, mtm: MainThreadMarker, state: &OverlayState) {
        if *state == OverlayState::Hidden || !self.pill_off_the_current_desktop(mtm) {
            return;
        }
        log::warn!("overlay: pill not on the current desktop, windows built again");
        let turn = Turn::next(&self.timer.pulse_generation);
        do_rebuild(mtm, &self.main);
        self.draw_state(mtm, state, turn);
        if self.pill_off_the_current_desktop(mtm) {
            log::warn!("overlay: pill still not on the current desktop after a rebuild");
        }
    }

    fn pill_off_the_current_desktop(&self, mtm: MainThreadMarker) -> bool {
        let pretended = self
            .main
            .get(mtm)
            .is_some_and(|main| main.off_desktop_pretended.replace(false));
        pretended
            || self.main.windows(mtm).iter().any(|ws| {
                let window = ws.window();
                window.isVisible() && !window.isOnActiveSpace()
            })
    }

    fn draw_state(&self, mtm: MainThreadMarker, state: &OverlayState, turn: Turn) {
        let commands_unavailable = self.timer.commands_unavailable.load(Ordering::SeqCst);
        let main = (&self.main, mtm);
        let ws_vec = self.main.windows(mtm);
        log::debug!("overlay: {state:?} shown on {} screen(s)", ws_vec.len());
        match state {
            OverlayState::Recording => {
                let shown = self.live_transcript.shown(state);
                apply_recording_state(
                    &self.main,
                    mtm,
                    turn,
                    self.feed.clone(),
                    commands_unavailable,
                );
                apply_live_transcript(&ws_vec, shown.as_ref());
            }
            OverlayState::RecognizingSpeech => {
                apply_processing_state(main, state, turn, commands_unavailable);
            }
            OverlayState::WaitingForFocus => {
                apply_waiting_for_focus_state(main, turn, commands_unavailable);
            }
            OverlayState::FailureNotice(notice) => {
                show_notice(main, notice, turn.generation, self.timer.clone());
            }
            OverlayState::Hidden => apply_hidden_state(&ws_vec, commands_unavailable),
        }
    }
}
