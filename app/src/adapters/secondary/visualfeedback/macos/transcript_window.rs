//! The glass text window under the pill: the live transcript while the recording runs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSColor, NSFont, NSLineBreakMode, NSTextField, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::glass::{GlassFinish, GlassPanel, ShadowStyle, SHADOW_MARGIN};
use super::motion::reduce_motion;
use crate::adapters::main_queue::{on_main_after, MainOnly};

const WIDTH: f64 = 480.0;
/// Space between the bottom of the pill and the top of the text window.
const GAP_BELOW_PILL: f64 = 8.0;
const PADDING_X: f64 = 14.0;
const PADDING_Y: f64 = 9.0;
const FONT_SIZE: f64 = 14.0;
const MAX_LINES: isize = 2;
const CORNER_RADIUS: f64 = 16.0;
const SHADOW: ShadowStyle =
    ShadowStyle { drop: 6.0, blur: 18.0, opacity: 0.15, room_above: SHADOW_MARGIN };
/// The window rises this far as it fades in.
const RISE: f64 = 6.0;
const RISE_SECS: f64 = 0.5;
const FRAME_MS: i64 = 16;

/// The window under the pill; touched on the main thread only.
pub(super) struct TranscriptWindow {
    panel: GlassPanel,
    pub(super) label: Retained<NSTextField>,
    /// Bumped by every rise-in and hide; a step of an older rise-in stops.
    rise: Arc<AtomicU64>,
}

impl TranscriptWindow {
    /// Builds the window centred under `pill`, hidden until it gets a text.
    pub(super) fn build(mtm: MainThreadMarker, pill: NSRect) -> Self {
        let panel = GlassPanel::build(mtm, glass_rect(pill, line_height()), CORNER_RADIUS, SHADOW);
        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(""), mtm);
        label.setFont(Some(&NSFont::systemFontOfSize(FONT_SIZE)));
        label.setTextColor(Some(&NSColor::labelColor()));
        label.setMaximumNumberOfLines(MAX_LINES);
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        label.setPreferredMaxLayoutWidth(WIDTH - 2.0 * PADDING_X);
        panel.content.addSubview(&label);
        Self { panel, label, rise: Arc::new(AtomicU64::new(0)) }
    }

    pub(super) fn window(&self) -> &NSWindow {
        &self.panel.window
    }

    pub(super) fn glass_finish(&self) -> GlassFinish {
        self.panel.finish()
    }

    /// Where the glass is, in screen coordinates, read from the window.
    pub(super) fn glass_rect(&self) -> NSRect {
        self.panel.glass_rect()
    }

    /// Shows `text` under the pill whose glass is `pill`, or hides the window for `None`.
    pub(super) fn show(&self, text: Option<&str>, pill: NSRect) {
        match text {
            Some(text) => self.show_text(text, pill),
            None => self.hide(),
        }
    }

    /// Shows `text` under the pill whose glass is `pill`, rising in when it was hidden.
    fn show_text(&self, text: &str, pill: NSRect) {
        self.label.setStringValue(&NSString::from_str(text));
        let text_height = self.text_height();
        self.label.setFrame(NSRect {
            origin: NSPoint { x: PADDING_X, y: PADDING_Y },
            size: NSSize { width: WIDTH - 2.0 * PADDING_X, height: text_height },
        });
        self.panel.set_glass_rect(glass_rect(pill, text_height));
        if self.panel.window.isVisible() {
            return;
        }
        self.panel.show();
        if !reduce_motion() {
            self.rise_in();
        }
    }

    pub(super) fn hide(&self) {
        self.rise.fetch_add(1, Ordering::SeqCst);
        self.panel.hide();
    }

    /// Sets the window's opacity, for a fade-out of the whole overlay.
    pub(super) fn set_opacity(&self, alpha: f64) {
        self.panel.window.setAlphaValue(alpha);
    }

    fn text_height(&self) -> f64 {
        let fitted = self.label.intrinsicContentSize().height;
        fitted.clamp(line_height(), line_height() * MAX_LINES as f64)
    }

    fn rise_in(&self) {
        let generation = self.rise.fetch_add(1, Ordering::SeqCst) + 1;
        let mtm = MainThreadMarker::from(&*self.panel.window);
        let window = MainOnly::new(mtm, self.panel.window.clone());
        let landing = self.panel.window.frame().origin;
        place(&self.panel.window, 0.0, landing);
        step_rise(
            window,
            landing,
            Instant::now(),
            generation,
            Arc::clone(&self.rise),
        );
    }
}

/// One frame of the rise-in: eases the window up and in until `RISE_SECS` have passed.
fn step_rise(
    window: MainOnly<Retained<NSWindow>>,
    landing: NSPoint,
    start: Instant,
    generation: u64,
    current: Arc<AtomicU64>,
) {
    on_main_after(FRAME_MS, move |mtm| {
        if current.load(Ordering::SeqCst) != generation {
            return;
        }
        let progress = (start.elapsed().as_secs_f64() / RISE_SECS).min(1.0);
        if let Some(shown) = window.get(mtm) {
            place(&shown, ease_out(progress), landing);
        }
        if progress < 1.0 {
            step_rise(window, landing, start, generation, current);
        }
    });
}

/// Cubic ease-out: fast at first, settling at the end.
fn ease_out(progress: f64) -> f64 {
    1.0 - (1.0 - progress).powi(3)
}

/// Shows `window` `eased` of the way in: as opaque, and as close to `landing`.
fn place(window: &NSWindow, eased: f64, landing: NSPoint) {
    window.setAlphaValue(eased);
    window.setFrameOrigin(NSPoint { x: landing.x, y: landing.y - RISE * (1.0 - eased) });
}

fn line_height() -> f64 {
    let font = NSFont::systemFontOfSize(FONT_SIZE);
    (font.ascender() - font.descender() + font.leading()).ceil()
}

/// The glass of a text `text_height` tall, centred under the pill whose glass is `pill`.
fn glass_rect(pill: NSRect, text_height: f64) -> NSRect {
    let height = text_height + 2.0 * PADDING_Y;
    NSRect {
        origin: NSPoint {
            x: pill.origin.x + pill.size.width / 2.0 - WIDTH / 2.0,
            y: pill.origin.y - GAP_BELOW_PILL - height,
        },
        size: NSSize { width: WIDTH, height },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_ease_out_from_nothing_to_whole() {
        assert!(
            (ease_out(0.0) - 0.0).abs() < 1e-6,
            "{} is not {}",
            ease_out(0.0),
            0.0
        );
        assert!(
            (ease_out(1.0) - 1.0).abs() < 1e-6,
            "{} is not {}",
            ease_out(1.0),
            1.0
        );
        assert!(ease_out(0.5) > 0.5, "the rise slows down towards its end");
    }

    #[test]
    fn should_hang_the_window_eight_points_under_the_pill() {
        let pill = NSRect {
            origin: NSPoint { x: 400.0, y: 800.0 },
            size: NSSize { width: 200.0, height: 40.0 },
        };
        let rect = glass_rect(pill, 20.0);
        assert_eq!(rect.origin, NSPoint { x: 260.0, y: 754.0 });
        assert!(
            (rect.size.height - 38.0).abs() < 1e-6,
            "{} is not {}",
            rect.size.height,
            38.0
        );
    }
}
