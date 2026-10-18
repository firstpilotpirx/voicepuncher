use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSColor, NSFont, NSTextField, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::window_set::floating_panel;

const WIDTH: f64 = 480.0;
const HEIGHT: f64 = 52.0;
/// Space between the bottom of the pill and the top of the text window.
const GAP_BELOW_PILL: f64 = 6.0;
const PADDING: f64 = 10.0;
const FONT_SIZE: f64 = 14.0;
const MAX_LINES: isize = 2;
const CORNER_RADIUS: f64 = 12.0;

/// The window under the pill that shows the live transcript while the user dictates.
pub(super) struct TranscriptWindow {
    pub(super) window: Retained<NSWindow>,
    pub(super) label: Retained<NSTextField>,
}

impl TranscriptWindow {
    /// Builds the window centred under `pill`, hidden until it gets a text.
    pub(super) fn build(mtm: MainThreadMarker, pill: NSRect) -> Self {
        let frame = NSRect {
            origin: NSPoint {
                x: pill.origin.x + (pill.size.width - WIDTH) / 2.0,
                y: pill.origin.y - GAP_BELOW_PILL - HEIGHT,
            },
            size: NSSize {
                width: WIDTH,
                height: HEIGHT,
            },
        };
        let (window, content) = floating_panel(mtm, frame, CORNER_RADIUS);

        let label = NSTextField::wrappingLabelWithString(&NSString::from_str(""), mtm);
        label.setFrame(NSRect {
            origin: NSPoint {
                x: PADDING,
                y: PADDING / 2.0,
            },
            size: NSSize {
                width: WIDTH - 2.0 * PADDING,
                height: HEIGHT - PADDING,
            },
        });
        label.setFont(Some(&NSFont::systemFontOfSize(FONT_SIZE)));
        label.setTextColor(Some(&NSColor::labelColor()));
        label.setMaximumNumberOfLines(MAX_LINES);
        content.addSubview(&label);

        Self { window, label }
    }

    /// Shows `text`, or hides the window for `None`.
    pub(super) fn show(&self, text: Option<&str>) {
        match text {
            Some(text) => {
                self.label.setStringValue(&NSString::from_str(text));
                self.window.orderFrontRegardless();
            }
            None => self.window.orderOut(None),
        }
    }
}
