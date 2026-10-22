use std::cell::RefCell;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2_app_kit::NSWindow;
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::glass::{GlassFinish, GlassPanel, ShadowStyle};
use super::motion::reduce_motion;
use super::pill_content::{GlassTint, PillKind};
use super::pill_view::{PillLook, PillView, PILL_HEIGHT};
use super::topology::ScreenTopology;
use super::transcript_window::TranscriptWindow;
use crate::adapters::main_queue::MainOnly;

/// The overlay windows of every screen.
pub(super) type Windows = Vec<Rc<WindowSet>>;

const PILL_CORNER_RADIUS: f64 = PILL_HEIGHT / 2.0;
/// The pill's width before its first state is drawn.
const INITIAL_PILL_WIDTH: f64 = 200.0;
/// Space between the top of the screen's visible area and the pill.
const GAP_BELOW_SCREEN_TOP: f64 = 4.0;
const PILL_SHADOW: ShadowStyle = ShadowStyle {
    drop: 8.0,
    blur: 24.0,
    opacity: 0.2,
    // WHY: more room above would reach past the top of the visible frame, and macOS would push
    // the pill down onto the text window
    room_above: GAP_BELOW_SCREEN_TOP,
};

/// The overlay of one screen: the glass pill and the text window under it.
pub(super) struct WindowSet {
    panel: GlassPanel,
    pub(super) pill: PillView,
    pub(super) transcript: TranscriptWindow,
}

impl WindowSet {
    pub(super) fn build(mtm: MainThreadMarker, visible_frame: NSRect) -> Rc<Self> {
        let pill_rect = NSRect {
            origin: NSPoint {
                x: visible_frame.origin.x + (visible_frame.size.width - INITIAL_PILL_WIDTH) / 2.0,
                y: visible_frame.origin.y + visible_frame.size.height
                    - PILL_HEIGHT
                    - GAP_BELOW_SCREEN_TOP,
            },
            size: NSSize { width: INITIAL_PILL_WIDTH, height: PILL_HEIGHT },
        };
        let panel = GlassPanel::build(mtm, pill_rect, PILL_CORNER_RADIUS, PILL_SHADOW);
        let pill = PillView::build(mtm, &panel.content);
        let transcript = TranscriptWindow::build(mtm, pill_rect);
        Rc::new(Self { panel, pill, transcript })
    }

    pub(super) fn window(&self) -> &NSWindow {
        &self.panel.window
    }

    pub(super) fn glass_finish(&self) -> GlassFinish {
        self.panel.finish()
    }

    /// Where the pill's glass is, in screen coordinates, read from its window.
    pub(super) fn glass_rect(&self) -> NSRect {
        self.panel.glass_rect()
    }

    /// Draws `look` on glass tinted `tint`, the pill as wide as it needs around the same
    /// centre, and brings it to the front; the text window stays as it is.
    pub(super) fn draw(
        &self,
        look: PillLook,
        tint: GlassTint,
        commands_unavailable: bool,
        visible_columns: usize,
    ) {
        self.panel.set_tint(tint);
        let width = self.pill.draw(
            look,
            commands_unavailable && look.kind != PillKind::UnavailableAlone,
            visible_columns,
            self.panel.has_light_content(),
            reduce_motion(),
        );
        self.set_pill_width(width);
        self.panel.show();
    }

    /// Shows `text` in the text window under the pill where it is now, or hides the window for
    /// `None`.
    pub(super) fn show_live_transcript(&self, text: Option<&str>) {
        self.transcript.show(text, self.panel.glass_rect());
    }

    /// Hides the pill and the text window and stops their motion.
    pub(super) fn hide(&self) {
        self.pill.stop();
        self.panel.hide();
        self.transcript.hide();
    }

    /// Sets the opacity of the pill and the text window, for a fade-out.
    pub(super) fn set_opacity(&self, alpha: f64) {
        self.panel.window.setAlphaValue(alpha);
        self.transcript.set_opacity(alpha);
    }

    /// Keeps the overlay above other windows on the current space.
    pub(super) fn keep_in_front(&self) {
        self.panel.show_again();
    }

    fn set_pill_width(&self, width: f64) {
        let pill = self.panel.glass_rect();
        self.panel.set_glass_rect(NSRect {
            origin: NSPoint {
                x: pill.origin.x + (pill.size.width - width) / 2.0,
                y: pill.origin.y,
            },
            size: NSSize { width, height: pill.size.height },
        });
    }
}

impl Drop for WindowSet {
    fn drop(&mut self) {
        self.hide();
    }
}

/// The overlay's part that lives on the main thread: the windows of every screen and the
/// screens they were built for.
pub(super) struct MainState {
    pub(super) windows: RefCell<Windows>,
    pub(super) topology: RefCell<ScreenTopology>,
}

const _: () = crate::assert_not_send!(WindowSet);
const _: () = crate::assert_not_send!(MainState);

/// The overlay's main-thread part, reached from any thread.
pub(super) type SharedWindows = MainOnly<MainState>;

impl MainOnly<MainState> {
    /// The windows of the moment; none once the overlay is gone.
    pub(super) fn windows(&self, mtm: MainThreadMarker) -> Windows {
        self.get(mtm)
            .map(|state| state.windows.borrow().clone())
            .unwrap_or_default()
    }
}
