//! Drawing the menu bar icon and its frames.

use super::{
    dispatch_to_main_after, Button, CALayer, CGPoint, CGRect, CGSize, IconPicture, NSArray,
    NSColor, NSFontWeightRegular, NSImage, NSImageSymbolConfiguration, NSString, NSWorkspace,
    Ordering, Retained, BUTTON, DOT_DIAMETER, DOT_INSET, FALLBACK_SYMBOL, FRAME_MS, ICONS,
    POINT_SIZE, PROCESSING_FRAMES, STILL_PROCESSING_FRAME,
};

pub(super) fn draw(picture: IconPicture, generation: u64) {
    let still = reduce_motion();
    let frame = picture.animated.then_some(if still {
        STILL_PROCESSING_FRAME
    } else {
        PROCESSING_FRAMES[0]
    });
    BUTTON.with_borrow(|button| {
        let Some(button) = button else { return };
        button.view.setImage(icon_image(picture, frame).as_deref());
        place_dot(button);
        button.dot.setHidden(!picture.red_dot);
    });
    if picture.animated && !still {
        next_frame(picture, generation, 1);
    }
}

/// Steps the processing animation until another icon is drawn.
pub(super) fn next_frame(picture: IconPicture, generation: u64, frame: usize) {
    dispatch_to_main_after(FRAME_MS, move || {
        if ICONS.load(Ordering::SeqCst) != generation {
            return;
        }
        let value = PROCESSING_FRAMES[frame % PROCESSING_FRAMES.len()];
        BUTTON.with_borrow(|button| {
            if let Some(button) = button {
                button
                    .view
                    .setImage(icon_image(picture, Some(value)).as_deref());
            }
        });
        next_frame(picture, generation, frame + 1);
    });
}

pub(super) fn reduce_motion() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// The symbol of `picture` at `frame`, or the fallback microphone when the system lacks it;
/// a template image follows the menu bar's ink, the others are drawn in the label colour.
pub(super) fn icon_image(picture: IconPicture, frame: Option<f64>) -> Option<Retained<NSImage>> {
    let image = symbol(picture.symbol, frame).or_else(|| symbol(FALLBACK_SYMBOL, None))?;
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    let weight = unsafe { NSFontWeightRegular };
    let mut config =
        NSImageSymbolConfiguration::configurationWithPointSize_weight(POINT_SIZE, weight);
    if !picture.template {
        let ink = NSColor::labelColor();
        let palette =
            NSImageSymbolConfiguration::configurationWithPaletteColors(&NSArray::from_slice(&[
                &*ink,
            ]));
        config = config.configurationByApplyingConfiguration(&palette);
    }
    let image = image.imageWithSymbolConfiguration(&config)?;
    image.setTemplate(picture.template);
    Some(image)
}

/// The SF Symbol `name`, at the variable value `frame` when given; its accessibility
/// description is its name, so a test can tell which symbol shows.
pub(super) fn symbol(name: &str, frame: Option<f64>) -> Option<Retained<NSImage>> {
    let name = NSString::from_str(name);
    match frame {
        Some(value) => NSImage::imageWithSystemSymbolName_variableValue_accessibilityDescription(
            &name,
            value,
            Some(&name),
        ),
        None => NSImage::imageWithSystemSymbolName_accessibilityDescription(&name, Some(&name)),
    }
}

pub(super) fn red_dot() -> Retained<CALayer> {
    let dot = CALayer::new();
    dot.setBackgroundColor(Some(&NSColor::systemRedColor().CGColor()));
    dot.setCornerRadius(DOT_DIAMETER / 2.0);
    dot.setHidden(true);
    dot
}

/// The red dot at the lower right corner of the button, which may be flipped.
pub(super) fn place_dot(button: &Button) {
    let bounds = button.view.bounds();
    let x = bounds.size.width - DOT_DIAMETER - DOT_INSET;
    let y = if button.view.isFlipped() {
        bounds.size.height - DOT_DIAMETER - DOT_INSET
    } else {
        DOT_INSET
    };
    button.dot.setFrame(CGRect::new(
        CGPoint::new(x, y),
        CGSize::new(DOT_DIAMETER, DOT_DIAMETER),
    ));
}
