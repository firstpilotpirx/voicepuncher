//! SF Symbols drawn by the overlay and the pointer badge.

use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSFontWeightMedium, NSImage, NSImageSymbolConfiguration};
use objc2_foundation::{NSArray, NSString};

/// The SF Symbol `name` at `point_size`, weight medium, to be tinted by its image view; its
/// accessibility description is its name, so a test can tell which symbol shows.
pub(super) fn symbol_image(name: &str, point_size: f64) -> Option<Retained<NSImage>> {
    let name = NSString::from_str(name);
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&name, Some(&name))?;
    image.imageWithSymbolConfiguration(&medium(point_size))
}

/// The SF Symbol `name` drawn in `palette`, one colour per layer of the symbol.
pub(super) fn palette_symbol_image(
    name: &str,
    point_size: f64,
    palette: &[&NSColor],
) -> Option<Retained<NSImage>> {
    let name = NSString::from_str(name);
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&name, Some(&name))?;
    let colours =
        NSImageSymbolConfiguration::configurationWithPaletteColors(&NSArray::from_slice(palette));
    let config = medium(point_size).configurationByApplyingConfiguration(&colours);
    image.imageWithSymbolConfiguration(&config)
}

/// The name of the symbol `image` draws.
pub(super) fn symbol_name(image: &NSImage) -> Option<String> {
    image
        .accessibilityDescription()
        .map(|name| name.to_string())
}

fn medium(point_size: f64) -> Retained<NSImageSymbolConfiguration> {
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    let weight = unsafe { NSFontWeightMedium };
    NSImageSymbolConfiguration::configurationWithPointSize_weight(point_size, weight)
}
