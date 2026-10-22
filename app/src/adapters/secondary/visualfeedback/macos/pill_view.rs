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

    /// Draws `look` and returns the width the pill needs for it; `visible_columns` is how many
    /// columns the waveform shows, `light_content` whether the glass is milky.
    #[expect(clippy::too_many_arguments, reason = "size baseline")]
    pub(super) fn draw(
        &self,
        look: PillLook,
        commands_unavailable: bool,
        visible_columns: usize,
        light_content: bool,
        reduce_motion: bool,
    ) -> f64 {
        let items = pill_items(look.kind, commands_unavailable);
        let sign = match look.kind {
            PillKind::UnavailableAlone => Some(COMMANDS_UNAVAILABLE_SIGN),
            _ => look.sign,
        };
        if let Some(sign) = sign {
            self.show_sign(sign, light_content);
        }
        self.colour_content(light_content);
        set_view_size(
            &self.waveform.view,
            waveform_width(visible_columns),
            WAVEFORM_HEIGHT,
        );
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
            unsafe { self.spinner.startAnimation(None) };
        } else {
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

fn show_symbol(view: &NSImageView, sign: SignLook, light_content: bool) {
    let image = match sign.tint {
        IconTint::FailureTriangle => {
            let (mark, triangle) = failure_palette(light_content);
            palette_symbol_image(sign.symbol, ICON_POINT_SIZE, &[&mark, &triangle])
        }
        _ => symbol_image(sign.symbol, ICON_POINT_SIZE),
    };
    view.setImage(image.as_deref());
    let tint = match sign.tint {
        IconTint::Content => Some(NSColor::labelColor()),
        IconTint::Unavailable => Some(NSColor::secondaryLabelColor()),
        IconTint::PastedGreen => Some(NSColor::systemGreenColor()),
        IconTint::FailureTriangle => None,
    };
    view.setContentTintColor(tint.as_deref());
}

/// The exclamation mark and the triangle: a dark mark in a yellow triangle over dark glass,
/// a light mark in a dark amber triangle over milky glass.
fn failure_palette(light_content: bool) -> (Retained<NSColor>, Retained<NSColor>) {
    if light_content {
        let (r, g, b) = DARK_AMBER;
        (
            NSColor::whiteColor(),
            NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0),
        )
    } else {
        (NSColor::blackColor(), NSColor::systemYellowColor())
    }
}

/// The colour of drawn content — columns, separator — matching the glass.
fn content_colour(light_content: bool) -> Retained<NSColor> {
    if light_content {
        let (r, g, b) = LIGHT_GLASS_CONTENT;
        NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
    } else {
        NSColor::whiteColor()
    }
}

fn visible_symbol(view: &NSImageView) -> Option<String> {
    if view.isHidden() {
        return None;
    }
    view.image().and_then(|image| symbol_name(&image))
}

fn label_text(text: &str, colour: &NSColor) -> Retained<NSAttributedString> {
    let weight = unsafe { NSFontWeightMedium };
    let font = NSFont::systemFontOfSize_weight(LABEL_FONT_SIZE, weight);
    let tracking = NSNumber::new_f64(LABEL_TRACKING);
    let keys = unsafe {
        [
            NSFontAttributeName,
            NSKernAttributeName,
            NSForegroundColorAttributeName,
        ]
    };
    let values: [&AnyObject; 3] = [&font, &tracking, colour];
    let attributes = NSDictionary::from_slices(&keys, &values);
    // SAFETY: each attribute holds the type its key documents: a font, a number, a colour.
    unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(text), &attributes) }
}

fn record_sign(mtm: MainThreadMarker) -> (Retained<NSView>, Retained<CALayer>) {
    let view = sized_view(mtm, RECORD_RING_SIZE, RECORD_RING_SIZE);
    let red = NSColor::systemRedColor().CGColor();
    let ring = CAShapeLayer::new();
    let inset = RECORD_RING_STROKE / 2.0;
    let circle = square(inset, RECORD_RING_SIZE - RECORD_RING_STROKE);
    // SAFETY: a null transform is documented as "no transform".
    let path = unsafe { CGPath::with_ellipse_in_rect(circle, std::ptr::null()) };
    ring.setPath(Some(&path));
    ring.setFillColor(None);
    ring.setStrokeColor(Some(&red));
    ring.setLineWidth(RECORD_RING_STROKE);
    let dot = CALayer::new();
    dot.setFrame(square(
        (RECORD_RING_SIZE - RECORD_DOT_SIZE) / 2.0,
        RECORD_DOT_SIZE,
    ));
    dot.setCornerRadius(RECORD_DOT_SIZE / 2.0);
    dot.setBackgroundColor(Some(&red));
    if let Some(layer) = view.layer() {
        layer.addSublayer(&ring);
        layer.addSublayer(&dot);
    }
    (view, dot)
}

fn image_view(mtm: MainThreadMarker) -> Retained<NSImageView> {
    let view = NSImageView::initWithFrame(mtm.alloc::<NSImageView>(), square(0.0, ICON_BOX));
    view.setWantsLayer(true);
    view
}

fn label(mtm: MainThreadMarker) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    label.setDrawsBackground(false);
    label
}

fn spinner(mtm: MainThreadMarker) -> Retained<NSProgressIndicator> {
    let spinner = NSProgressIndicator::initWithFrame(
        mtm.alloc::<NSProgressIndicator>(),
        square(0.0, SPINNER_SIZE),
    );
    spinner.setStyle(NSProgressIndicatorStyle::Spinning);
    spinner.setIndeterminate(true);
    spinner.setControlSize(NSControlSize::Small);
    spinner.setDisplayedWhenStopped(false);
    spinner
}

fn separator(mtm: MainThreadMarker) -> Retained<NSView> {
    sized_view(mtm, SEPARATOR_WIDTH, SEPARATOR_HEIGHT)
}

fn sized_view(mtm: MainThreadMarker, width: f64, height: f64) -> Retained<NSView> {
    let view = NSView::new(mtm);
    view.setWantsLayer(true);
    set_view_size(&view, width, height);
    view
}

fn set_view_size(view: &NSView, width: f64, height: f64) {
    view.setFrameSize(NSSize { width, height });
}

fn square(offset: f64, side: f64) -> NSRect {
    NSRect { origin: NSPoint { x: offset, y: offset }, size: NSSize { width: side, height: side } }
}
