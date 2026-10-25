//! The items of the overlay pill — record sign, waveform, icon, label, spinner, separator and
//! the muted microphone — and how they line up in one row.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSColor, NSControlSize, NSFont, NSFontAttributeName, NSFontWeightMedium,
    NSForegroundColorAttributeName, NSImageView, NSKernAttributeName, NSProgressIndicator,
    NSProgressIndicatorStyle, NSTextField, NSView,
};
use objc2_core_graphics::CGPath;
use objc2_foundation::{
    NSAttributedString, NSDictionary, NSNumber, NSPoint, NSRect, NSSize, NSString,
};
use objc2_quartz_core::{CALayer, CAShapeLayer};

use super::motion::{is_breathing, is_pulsing, set_breathing, set_pulsing};
use super::pill_content::{
    item_gap, pill_items, pill_width, row_width, IconTint, PillItem, PillKind, SignLook,
    COMMANDS_UNAVAILABLE_SIGN, ICON_BOX, RECORD_RING_SIZE, SEPARATOR_WIDTH, SPINNER_SIZE,
};
use super::symbols::{palette_symbol_image, symbol_image, symbol_name};
use super::waveform::{waveform_width, WaveformView};

mod parts;
use parts::{
    content_colour, image_view, label, label_text, record_sign, separator, set_view_size,
    show_symbol, spinner, visible_symbol,
};

pub(super) const PILL_HEIGHT: f64 = 56.0;
const RECORD_RING_STROKE: f64 = 2.0;
const RECORD_DOT_SIZE: f64 = 18.0;
const WAVEFORM_HEIGHT: f64 = 28.0;
const ICON_POINT_SIZE: f64 = 21.0;
const LABEL_FONT_SIZE: f64 = 13.0;
const LABEL_TRACKING: f64 = 0.5;
const SEPARATOR_HEIGHT: f64 = 26.0;
const SEPARATOR_ALPHA: f64 = 0.3;
/// Dark content over milky glass, `#1D1D1F`.
const LIGHT_GLASS_CONTENT: (f64, f64, f64) = (29.0 / 255.0, 29.0 / 255.0, 31.0 / 255.0);
/// The failure triangle over milky glass, dark amber `#C27A00`.
const DARK_AMBER: (f64, f64, f64) = (194.0 / 255.0, 122.0 / 255.0, 0.0);

/// What the pill draws: the kind of row, its icon and label, and whether the icon breathes.
#[derive(Debug, Clone, Copy)]
pub(super) struct PillLook {
    pub(super) kind: PillKind,
    pub(super) sign: Option<SignLook>,
    pub(super) breathes: bool,
}

/// What the pill is drawn in: whether commands are unavailable, how many columns the
/// waveform shows, whether the glass is milky and whether motion is reduced.
#[derive(Debug, Clone, Copy)]
pub(super) struct PillCanvas {
    pub(super) commands_unavailable: bool,
    pub(super) visible_columns: usize,
    pub(super) light_content: bool,
    pub(super) reduce_motion: bool,
}

/// The pill's views; touched on the main thread only.
pub(super) struct PillView {
    record_ring: Retained<NSView>,
    record_dot: Retained<CALayer>,
    pub(super) waveform: WaveformView,
    icon: Retained<NSImageView>,
    label: Retained<NSTextField>,
    spinner: Retained<NSProgressIndicator>,
    separator: Retained<NSView>,
    unavailable_icon: Retained<NSImageView>,
}

impl PillView {
    /// Builds the items into `parent`, every one hidden.
    pub(super) fn build(mtm: MainThreadMarker, parent: &NSView) -> Self {
        let (record_ring, record_dot) = record_sign(mtm);
        let pill = Self {
            record_ring,
            record_dot,
            waveform: WaveformView::build(mtm),
            icon: image_view(mtm),
            label: label(mtm),
            spinner: spinner(mtm),
            separator: separator(mtm),
            unavailable_icon: image_view(mtm),
        };
        for item in pill.views() {
            item.setHidden(true);
            parent.addSubview(item);
        }
        pill
    }

    /// Draws `look` in `canvas` and returns the width the pill needs for it.
    pub(super) fn draw(&self, look: PillLook, canvas: PillCanvas) -> f64 {
        let items = pill_items(look.kind, canvas.commands_unavailable);
        let sign = match look.kind {
            PillKind::UnavailableAlone => Some(COMMANDS_UNAVAILABLE_SIGN),
            _ => look.sign,
        };
        if let Some(sign) = sign {
            self.show_sign(sign, canvas.light_content);
        }
        self.colour_content(canvas.light_content);
        set_view_size(
            &self.waveform.view,
            waveform_width(canvas.visible_columns),
            WAVEFORM_HEIGHT,
        );
        let reduce_motion = canvas.reduce_motion;
        self.animate(&items, look.breathes && !reduce_motion, reduce_motion);
        self.arrange(look.kind, &items)
    }

    /// Stops every animation of a hidden pill.
    pub(super) fn stop(&self) {
        self.animate(&[], false, true);
    }

    /// The symbol of the icon on the left of the label, while it shows.
    pub(super) fn icon_symbol(&self) -> Option<String> {
        visible_symbol(&self.icon)
    }

    /// The label, while it shows.
    pub(super) fn label_text(&self) -> Option<String> {
        (!self.label.isHidden()).then(|| self.label.stringValue().to_string())
    }

    pub(super) fn shows_record_sign(&self) -> bool {
        !self.record_ring.isHidden()
    }

    pub(super) fn shows_commands_unavailable(&self) -> bool {
        visible_symbol(&self.unavailable_icon).is_some()
    }

    pub(super) fn record_dot_pulses(&self) -> bool {
        is_pulsing(&self.record_dot)
    }

    pub(super) fn icon_breathes(&self) -> bool {
        self.icon.layer().is_some_and(|layer| is_breathing(&layer))
    }

    fn views(&self) -> [&NSView; 7] {
        [
            &self.record_ring,
            &self.waveform.view,
            &self.icon,
            &self.label,
            &self.spinner,
            &self.separator,
            &self.unavailable_icon,
        ]
    }

    fn view_of(&self, item: PillItem) -> &NSView {
        match item {
            PillItem::RecordSign => &self.record_ring,
            PillItem::Waveform => &self.waveform.view,
            PillItem::Icon => &self.icon,
            PillItem::Label => &self.label,
            PillItem::Spinner => &self.spinner,
            PillItem::Separator => &self.separator,
            PillItem::UnavailableIcon => &self.unavailable_icon,
        }
    }

    fn show_sign(&self, sign: SignLook, light_content: bool) {
        // WHY: the muted microphone has a view of its own, drawn by `colour_content`
        if sign != COMMANDS_UNAVAILABLE_SIGN {
            show_symbol(&self.icon, sign, light_content);
        }
        let colour = match sign.tint {
            IconTint::Unavailable => NSColor::secondaryLabelColor(),
            _ => NSColor::labelColor(),
        };
        self.label
            .setAttributedStringValue(&label_text(sign.label, &colour));
        self.label.sizeToFit();
    }

    fn colour_content(&self, light_content: bool) {
        let content = content_colour(light_content);
        self.waveform.set_colour(&content);
        if let Some(layer) = self.separator.layer() {
            let line = content.colorWithAlphaComponent(SEPARATOR_ALPHA);
            layer.setBackgroundColor(Some(&line.CGColor()));
        }
        show_symbol(
            &self.unavailable_icon,
            COMMANDS_UNAVAILABLE_SIGN,
            light_content,
        );
    }

    fn animate(&self, items: &[PillItem], breathing: bool, reduce_motion: bool) {
        let recording = items.contains(&PillItem::RecordSign);
        set_pulsing(&self.record_dot, recording && !reduce_motion);
        if let Some(layer) = self.icon.layer() {
            set_breathing(&layer, breathing);
        }
        if items.contains(&PillItem::Spinner) {
            // SAFETY: a main-thread view built with a MainThreadMarker; a nil sender is allowed.
            unsafe { self.spinner.startAnimation(None) };
        } else {
            // SAFETY: a main-thread view built with a MainThreadMarker; a nil sender is allowed.
            unsafe { self.spinner.stopAnimation(None) };
        }
    }

    /// Shows only `items`, lined up and centred in the pill; returns the pill's width.
    fn arrange(&self, kind: PillKind, items: &[PillItem]) -> f64 {
        for view in self.views() {
            view.setHidden(true);
        }
        let widths: Vec<f64> = items
            .iter()
            .map(|item| self.view_of(*item).frame().size.width)
            .collect();
        let gap = item_gap(kind);
        let row = row_width(&widths, gap);
        let width = pill_width(kind, row);
        let mut x = (width - row) / 2.0;
        for (item, item_width) in items.iter().zip(widths) {
            let view = self.view_of(*item);
            let height = view.frame().size.height;
            view.setFrameOrigin(NSPoint { x, y: ((PILL_HEIGHT - height) / 2.0).round() });
            view.setHidden(false);
            x += item_width + gap;
        }
        width
    }
}
