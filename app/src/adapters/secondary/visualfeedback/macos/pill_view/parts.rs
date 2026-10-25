//! The symbols, labels, colours and small views a pill is drawn with.

use super::{
    palette_symbol_image, symbol_image, symbol_name, AnyObject, CALayer, CAShapeLayer, CGPath,
    IconTint, MainThreadMarker, NSAttributedString, NSColor, NSControlSize, NSDictionary, NSFont,
    NSFontAttributeName, NSFontWeightMedium, NSForegroundColorAttributeName, NSImageView,
    NSKernAttributeName, NSNumber, NSPoint, NSProgressIndicator, NSProgressIndicatorStyle, NSRect,
    NSSize, NSString, NSTextField, NSView, Retained, SignLook, DARK_AMBER, ICON_BOX,
    ICON_POINT_SIZE, LABEL_FONT_SIZE, LABEL_TRACKING, LIGHT_GLASS_CONTENT, RECORD_DOT_SIZE,
    RECORD_RING_SIZE, RECORD_RING_STROKE, SEPARATOR_HEIGHT, SEPARATOR_WIDTH, SPINNER_SIZE,
};

pub(super) fn show_symbol(view: &NSImageView, sign: SignLook, light_content: bool) {
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
pub(super) fn failure_palette(light_content: bool) -> (Retained<NSColor>, Retained<NSColor>) {
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
pub(super) fn content_colour(light_content: bool) -> Retained<NSColor> {
    if light_content {
        let (r, g, b) = LIGHT_GLASS_CONTENT;
        NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0)
    } else {
        NSColor::whiteColor()
    }
}

pub(super) fn visible_symbol(view: &NSImageView) -> Option<String> {
    if view.isHidden() {
        return None;
    }
    view.image().and_then(|image| symbol_name(&image))
}

pub(super) fn label_text(text: &str, colour: &NSColor) -> Retained<NSAttributedString> {
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    let weight = unsafe { NSFontWeightMedium };
    let font = NSFont::systemFontOfSize_weight(LABEL_FONT_SIZE, weight);
    let tracking = NSNumber::new_f64(LABEL_TRACKING);
    // SAFETY: an immutable framework constant, set before any Rust code runs.
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

pub(super) fn record_sign(mtm: MainThreadMarker) -> (Retained<NSView>, Retained<CALayer>) {
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

pub(super) fn image_view(mtm: MainThreadMarker) -> Retained<NSImageView> {
    let view = NSImageView::initWithFrame(mtm.alloc::<NSImageView>(), square(0.0, ICON_BOX));
    view.setWantsLayer(true);
    view
}

pub(super) fn label(mtm: MainThreadMarker) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    label.setDrawsBackground(false);
    label
}

pub(super) fn spinner(mtm: MainThreadMarker) -> Retained<NSProgressIndicator> {
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

pub(super) fn separator(mtm: MainThreadMarker) -> Retained<NSView> {
    sized_view(mtm, SEPARATOR_WIDTH, SEPARATOR_HEIGHT)
}

pub(super) fn sized_view(mtm: MainThreadMarker, width: f64, height: f64) -> Retained<NSView> {
    let view = NSView::new(mtm);
    view.setWantsLayer(true);
    set_view_size(&view, width, height);
    view
}

pub(super) fn set_view_size(view: &NSView, width: f64, height: f64) {
    view.setFrameSize(NSSize { width, height });
}

pub(super) fn square(offset: f64, side: f64) -> NSRect {
    NSRect { origin: NSPoint { x: offset, y: offset }, size: NSSize { width: side, height: side } }
}
