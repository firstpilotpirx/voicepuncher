//! The layers, views and window a glass panel is built from.

use super::{
    every_space, kCACornerCurveContinuous, AnyClass, CALayer, CFRetained, CGMutablePath, GlassTint,
    MainThreadMarker, NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua,
    NSBackingStoreType, NSColor, NSGlassEffectView, NSGlassEffectViewStyle, NSRect, NSSize, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindow, NSWindowStyleMask, Retained, ShadowStyle, Surface, FAILURE_RIM_ALPHA,
    FAILURE_TINT_ALPHA, GLASS_CLASS, OVERLAY_LEVEL, RIM_WHITE_ALPHA, RIM_WIDTH, TINTED_RIM_RGB,
    WAITING_RIM_ALPHA, WAITING_TINT_ALPHA,
};

/// The area of `whole` outside the rounded `glass`, so the shadow never darkens the glass.
pub(super) fn outside(
    whole: NSRect,
    glass: NSRect,
    corner_radius: f64,
) -> CFRetained<CGMutablePath> {
    let path = CGMutablePath::new();
    // SAFETY: a null transform is documented as "no transform"; the path is fresh and ours.
    unsafe { CGMutablePath::add_rect(Some(&path), std::ptr::null(), whole) };
    // SAFETY: as above.
    unsafe {
        CGMutablePath::add_rounded_rect(
            Some(&path),
            std::ptr::null(),
            glass,
            corner_radius,
            corner_radius,
        );
    }
    path
}

pub(super) fn tint_colours(tint: GlassTint) -> (Option<Retained<NSColor>>, Retained<NSColor>) {
    let (r, g, b) = TINTED_RIM_RGB;
    match tint {
        GlassTint::Clear => (None, NSColor::colorWithWhite_alpha(1.0, RIM_WHITE_ALPHA)),
        GlassTint::Waiting => (
            Some(NSColor::systemOrangeColor().colorWithAlphaComponent(WAITING_TINT_ALPHA)),
            NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, WAITING_RIM_ALPHA),
        ),
        GlassTint::Failure => (
            Some(NSColor::systemOrangeColor().colorWithAlphaComponent(FAILURE_TINT_ALPHA)),
            NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, FAILURE_RIM_ALPHA),
        ),
    }
}

pub(super) fn build_surface(mtm: MainThreadMarker, corner_radius: f64) -> Surface {
    if AnyClass::get(GLASS_CLASS).is_some() {
        let glass = NSGlassEffectView::new(mtm);
        glass.setCornerRadius(corner_radius);
        glass.setStyle(NSGlassEffectViewStyle::Clear);
        return Surface::System(glass);
    }
    let effect = NSVisualEffectView::new(mtm);
    effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
    effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    effect.setState(NSVisualEffectState::Active);
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    if let Some(dark) = NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }) {
        effect.setAppearance(Some(&dark));
    }
    effect.setWantsLayer(true);
    if let Some(layer) = effect.layer() {
        round(&layer, corner_radius);
        layer.setMasksToBounds(true);
    }
    Surface::Fallback(effect)
}

pub(super) fn build_rim(mtm: MainThreadMarker, corner_radius: f64) -> Retained<NSView> {
    let rim = layer_view(mtm);
    if let Some(layer) = rim.layer() {
        round(&layer, corner_radius);
        layer.setBorderWidth(RIM_WIDTH);
        let rim_colour = NSColor::colorWithWhite_alpha(1.0, RIM_WHITE_ALPHA);
        layer.setBorderColor(Some(&rim_colour.CGColor()));
    }
    rim
}

pub(super) fn cast_shadow(layer: &CALayer, shadow: ShadowStyle) {
    layer.setShadowColor(Some(&NSColor::blackColor().CGColor()));
    layer.setShadowOpacity(shadow.opacity);
    layer.setShadowRadius(shadow.blur / 2.0);
    layer.setShadowOffset(NSSize { width: 0.0, height: -shadow.drop });
}

pub(super) fn round(layer: &CALayer, corner_radius: f64) {
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    layer.setCornerCurve(unsafe { kCACornerCurveContinuous });
    layer.setCornerRadius(corner_radius);
}

pub(super) fn layer_view(mtm: MainThreadMarker) -> Retained<NSView> {
    let view = NSView::new(mtm);
    view.setWantsLayer(true);
    view
}

/// A borderless, see-through window on every space that lets the mouse through, on the
/// overlay level without a shadow.
pub(in crate::adapters::secondary::visualfeedback::macos) fn click_through_window(
    mtm: MainThreadMarker,
    frame: NSRect,
) -> Retained<NSWindow> {
    // SAFETY: a fresh main-thread allocation (mtm), initialised once with plain values.
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSWindow>(),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    // SAFETY: a Retained owns the window, so AppKit must not release it again on close.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setLevel(OVERLAY_LEVEL);
    window.setOpaque(false);
    window.setHasShadow(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setIgnoresMouseEvents(true);
    window.setCollectionBehavior(every_space());
    window
}
