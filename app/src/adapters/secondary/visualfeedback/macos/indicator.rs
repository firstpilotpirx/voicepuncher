//! The overlay as the pipeline's visual indicator.

use super::{
    apply_command_sign, apply_pasted_sign, command_sign, do_rebuild, fade_out_then_hide, lock,
    on_main, on_main_after, state_after_sign, Arc, AtomicBool, FailureNotice, LevelWave,
    LiveTranscriptView, MacosLiveTranscriptView, MacosVisualIndicator, MainThreadMarker,
    OverlayState, Redraw, SharedWindows, Turn, VisualIndicator, VoiceCommand, COMMAND_SIGN_MS,
    PASTED_SIGN_MS,
};

impl VisualIndicator for MacosVisualIndicator {
    fn show_recording(&self) {
        self.live_transcript.clear();
        *lock(&self.feed.wave) = LevelWave::starting_now();
        self.show_state(OverlayState::Recording);
    }

    fn show_recognizing_speech(&self) {
        self.show_state(OverlayState::RecognizingSpeech);
    }

    fn show_waiting_for_focus(&self) {
        self.show_state(OverlayState::WaitingForFocus);
    }

    fn hide(&self) {
        self.hide_overlay();
    }

    fn show_pasted(&self) {
        let turn = Turn::next(&self.pulse_generation);
        let main = self.main.clone();
        let commands_unavailable = Arc::clone(&self.commands_unavailable);
        // The sign hides itself, so a screen change draws the overlay hidden.
        *lock(&self.state) = OverlayState::Hidden;
        log::debug!("overlay: Pasted sign requested");

        on_main(move |mtm| {
            if !turn.is_current() {
                return;
            }
            apply_pasted_sign(&main.windows(mtm));
            on_main_after(PASTED_SIGN_MS, move |_| {
                fade_out_if_current(main, turn, commands_unavailable);
            });
        });
    }

    fn show_failure_notice(&self, notice: &FailureNotice) {
        self.show_state(OverlayState::FailureNotice(notice.clone()));
    }

    fn show_command_sign(&self, command: VoiceCommand) {
        let Some(sign) = command_sign(command) else {
            return;
        };
        let turn = Turn::next(&self.pulse_generation);
        let redraw = self.redraw();
        // The state stays what the command left, so a screen change redraws that state.
        let after = state_after_sign(&lock(&self.state));
        log::debug!("overlay: command sign {} requested", sign.label);

        on_main(move |mtm| {
            do_rebuild(mtm, &redraw.main);
            apply_command_sign(&redraw.main.windows(mtm), sign);
            on_main_after(COMMAND_SIGN_MS, move |mtm| {
                redraw_if_current(&redraw, mtm, &after, turn);
            });
        });
    }

    fn show_commands_unavailable(&self) {
        self.set_commands_unavailable(true);
    }

    fn hide_commands_unavailable(&self) {
        self.set_commands_unavailable(false);
    }

    fn live_transcript_view(&self) -> Box<dyn LiveTranscriptView> {
        Box::new(MacosLiveTranscriptView {
            state: Arc::clone(&self.state),
            main: self.main.clone(),
            live_transcript: self.live_transcript.clone(),
        })
    }
}

/// Fades the Pasted sign out, unless a later signal took over the overlay: its own drawing
/// stands.
fn fade_out_if_current(main: SharedWindows, turn: Turn, commands_unavailable: Arc<AtomicBool>) {
    if turn.is_current() {
        fade_out_then_hide(main, turn, commands_unavailable);
    }
}

/// Draws `state` after a command sign, unless a later signal took over the overlay: its own
/// drawing stands.
fn redraw_if_current(redraw: &Redraw, mtm: MainThreadMarker, state: &OverlayState, turn: Turn) {
    if turn.is_current() {
        redraw.draw(mtm, state, turn);
    }
}
