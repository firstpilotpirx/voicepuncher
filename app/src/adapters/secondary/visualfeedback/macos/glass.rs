//! A borderless, click-through window of frosted glass with a soft shadow around it: the
//! system's clear `NSGlassEffectView` on macOS 26 and later, which draws its own edge, and
//! elsewhere an `NSVisualEffectView` HUD material with a light rim of ours.

use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSBackingStoreType, NSColor, NSGlassEffectView, NSGlassEffectViewStyle, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{CGMutablePath, CGPath};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize};
use objc2_quartz_core::{kCACornerCurveContinuous, kCAFillRuleEvenOdd, CALayer, CAShapeLayer};

use super::pill_content::GlassTint;

/// Above app windows, below the pointer badge.
pub(super) const OVERLAY_LEVEL: isize = 3;
/// Room beside and under the glass for its shadow: the shadow's drop plus its blur.
pub(super) const SHADOW_MARGIN: f64 = 32.0;
const RIM_WIDTH: f64 = 1.0;
const RIM_WHITE_ALPHA: f64 = 0.6;
/// The warm rim of tinted glass, `#FFD696`.
const TINTED_RIM_RGB: (f64, f64, f64) = (1.0, 214.0 / 255.0, 150.0 / 255.0);
const WAITING_RIM_ALPHA: f64 = 0.8;
const FAILURE_RIM_ALPHA: f64 = 0.7;
const WAITING_TINT_ALPHA: f64 = 0.42;
const FAILURE_TINT_ALPHA: f64 = 0.26;
const GLASS_CLASS: &std::ffi::CStr = c"NSGlassEffectView";

/// How the glass casts its shadow.
#[derive(Debug, Clone, Copy)]
pub(super) struct ShadowStyle {
    pub(super) drop: f64,
    pub(super) blur: f64,
    pub(super) opacity: f32,
    /// Room the window leaves above the glass; the shadow falls downward, so it may be less
    /// than the room beside and under it.
    pub(super) room_above: f64,
}

/// How a glass panel is finished around its glass, as the tests see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlassFinish {
    /// Whether a light line is drawn over the glass along its top edge.
    pub top_highlight: bool,
    /// Whether our own rim is drawn around the glass.
    pub own_rim: bool,
    /// Whether the system glass is in its clear style; `None` on the fallback material.
    pub clear_system_glass: Option<bool>,
}

/// The glass the content sits on.
enum Surface {
    /// macOS 26 Liquid Glass: tinted by the system, adapts its content to what is under it.
    System(Retained<NSGlassEffectView>),
    /// Earlier macOS: the dark HUD material, tinted by the content view's background.
    Fallback(Retained<NSVisualEffectView>),
}

impl Surface {
    fn view(&self) -> &NSView {
        match self {
            Self::System(glass) => glass,
            Self::Fallback(effect) => effect,
        }
    }
}

/// A glass window; touched on the main thread only.
pub(super) struct GlassPanel {
    pub(super) window: Retained<NSWindow>,
    surface: Surface,
    /// Where the panel's items go, filling the glass.
    pub(super) content: Retained<NSView>,
    /// Our rim around the fallback material; the system glass draws its own edge.
    rim: Option<Retained<NSView>>,
    shadow: Retained<NSView>,
    shadow_mask: Retained<CAShapeLayer>,
    corner_radius: f64,
    room_above: f64,
}

impl GlassPanel {
    /// A hidden panel whose glass covers `glass_rect`, in screen coordinates.
    pub(super) fn build(
        mtm: MainThreadMarker,
        glass_rect: NSRect,
        corner_radius: f64,
        shadow: ShadowStyle,
    ) -> Self {
        let window = click_through_window(mtm, around(glass_rect, shadow.room_above));
        let root = NSView::initWithFrame(mtm.alloc::<NSView>(), at_origin(window.frame().size));
        root.setWantsLayer(true);
        let shadow_view = layer_view(mtm);
        let shadow_mask = CAShapeLayer::new();
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        shadow_mask.setFillRule(unsafe { kCAFillRuleEvenOdd });
        if let Some(layer) = shadow_view.layer() {
            cast_shadow(&layer, shadow);
            // SAFETY: the mask is a fresh layer used by this layer alone.
            unsafe { layer.setMask(Some(&shadow_mask)) };
        }
        let surface = build_surface(mtm, corner_radius);
        let content = layer_view(mtm);
        let rim = match &surface {
            Surface::System(glass) => {
                glass.setContentView(Some(&content));
                None
            }
            Surface::Fallback(effect) => {
                effect.addSubview(&content);
                Some(build_rim(mtm, corner_radius))
            }
        };
        root.addSubview(&shadow_view);
        root.addSubview(surface.view());
        if let Some(rim) = &rim {
            root.addSubview(rim);
        }
        window.setContentView(Some(&root));
        let panel = Self {
            window,
            surface,
            content,
            rim,
            shadow: shadow_view,
            shadow_mask,
            corner_radius,
            room_above: shadow.room_above,
        };
        panel.lay_out(glass_rect.size);
        panel
    }

    /// Where the glass is, in screen coordinates.
    pub(super) fn glass_rect(&self) -> NSRect {
        let frame = self.window.frame();
        NSRect {
            origin: NSPoint {
                x: frame.origin.x + SHADOW_MARGIN,
                y: frame.origin.y + SHADOW_MARGIN,
            },
            size: NSSize {
                width: frame.size.width - 2.0 * SHADOW_MARGIN,
                height: frame.size.height - SHADOW_MARGIN - self.room_above,
            },
        }
    }

    /// Moves and resizes the glass to `glass_rect`, in screen coordinates.
    pub(super) fn set_glass_rect(&self, glass_rect: NSRect) {
        if self.glass_rect() == glass_rect {
            return;
        }
        self.window
            .setFrame_display(around(glass_rect, self.room_above), true);
        self.lay_out(glass_rect.size);
    }

    /// Tints the glass and its rim for `tint`.
    pub(super) fn set_tint(&self, tint: GlassTint) {
        let (fill, rim) = tint_colours(tint);
        match &self.surface {
            Surface::System(glass) => glass.setTintColor(fill.as_deref()),
            Surface::Fallback(_) => {
                if let Some(layer) = self.content.layer() {
                    layer.setBackgroundColor(fill.map(|fill| fill.CGColor()).as_deref());
                }
            }
        }
        if let Some(layer) = self.rim.as_ref().and_then(|own_rim| own_rim.layer()) {
            layer.setBorderColor(Some(&rim.CGColor()));
        }
    }

    /// Shows the panel, opaque, over every app without taking the focus.
    pub(super) fn show(&self) {
        self.window.setAlphaValue(1.0);
        self.show_again();
    }

    /// Brings the shown panel back over every app on the current space.
    pub(super) fn show_again(&self) {
        self.window.setLevel(OVERLAY_LEVEL);
        self.window.orderFrontRegardless();
        self.window.setCollectionBehavior(every_space());
    }

    pub(super) fn hide(&self) {
        self.window.orderOut(None);
    }

    /// Whether the content shows dark on milky glass rather than light on dark glass.
    pub(super) fn has_light_content(&self) -> bool {
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        let names = NSArray::from_slice(&[unsafe { NSAppearanceNameAqua }, unsafe {
            NSAppearanceNameDarkAqua
        }]);
        let best = self
            .content
            .effectiveAppearance()
            .bestMatchFromAppearancesWithNames(&names);
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        best.is_some_and(|name| &*name == unsafe { NSAppearanceNameAqua })
    }

    /// How the glass is finished: its rim, a top line over it and the system glass style.
    pub(super) fn finish(&self) -> GlassFinish {
        let rim_layer = self.rim.as_ref().and_then(|rim| rim.layer());
        GlassFinish {
            top_highlight: rim_layer
                .as_ref()
                // SAFETY: the sublayers are only counted, on the main thread that owns them.
                .and_then(|layer| unsafe { layer.sublayers() })
                .is_some_and(|sublayers| !sublayers.is_empty()),
            own_rim: rim_layer.is_some_and(|layer| layer.borderWidth() > 0.0),
            clear_system_glass: match &self.surface {
                Surface::System(glass) => Some(glass.style() == NSGlassEffectViewStyle::Clear),
                Surface::Fallback(_) => None,
            },
        }
    }

    fn lay_out(&self, glass_size: NSSize) {
        let glass =
            NSRect { origin: NSPoint { x: SHADOW_MARGIN, y: SHADOW_MARGIN }, size: glass_size };
        let whole = at_origin(around(glass, self.room_above).size);
        self.shadow.setFrame(whole);
        self.surface.view().setFrame(glass);
        self.content.setFrame(at_origin(glass_size));
        if let Some(rim) = &self.rim {
            rim.setFrame(glass);
        }
        // SAFETY: a null transform is documented as "no transform".
        let glass_path = unsafe {
            CGPath::with_rounded_rect(
                glass,
                self.corner_radius,
                self.corner_radius,
                std::ptr::null(),
            )
        };
        if let Some(layer) = self.shadow.layer() {
            layer.setShadowPath(Some(&glass_path));
        }
        self.shadow_mask.setFrame(whole);
        self.shadow_mask
            .setPath(Some(&outside(whole, glass, self.corner_radius)));
    }
}

/// The area of `whole` outside the rounded `glass`, so the shadow never darkens the glass.
fn outside(whole: NSRect, glass: NSRect, corner_radius: f64) -> CFRetained<CGMutablePath> {
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

fn tint_colours(tint: GlassTint) -> (Option<Retained<NSColor>>, Retained<NSColor>) {
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

fn build_surface(mtm: MainThreadMarker, corner_radius: f64) -> Surface {
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

fn build_rim(mtm: MainThreadMarker, corner_radius: f64) -> Retained<NSView> {
    let rim = layer_view(mtm);
    if let Some(layer) = rim.layer() {
        round(&layer, corner_radius);
        layer.setBorderWidth(RIM_WIDTH);
        let rim_colour = NSColor::colorWithWhite_alpha(1.0, RIM_WHITE_ALPHA);
        layer.setBorderColor(Some(&rim_colour.CGColor()));
    }
    rim
}

fn cast_shadow(layer: &CALayer, shadow: ShadowStyle) {
    layer.setShadowColor(Some(&NSColor::blackColor().CGColor()));
    layer.setShadowOpacity(shadow.opacity);
    layer.setShadowRadius(shadow.blur / 2.0);
    layer.setShadowOffset(NSSize { width: 0.0, height: -shadow.drop });
}

fn round(layer: &CALayer, corner_radius: f64) {
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    layer.setCornerCurve(unsafe { kCACornerCurveContinuous });
    layer.setCornerRadius(corner_radius);
}

fn layer_view(mtm: MainThreadMarker) -> Retained<NSView> {
    let view = NSView::new(mtm);
    view.setWantsLayer(true);
    view
}

fn click_through_window(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSWindow> {
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

/// On every space, beside full-screen apps, left out of window cycling.
pub(super) fn every_space() -> NSWindowCollectionBehavior {
    NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle
        | NSWindowCollectionBehavior::FullScreenAuxiliary
}

/// The window frame around `glass`, with room for the shadow.
/// The window around `glass`: the shadow's room beside and under it, `room_above` above it.
fn around(glass: NSRect, room_above: f64) -> NSRect {
    NSRect {
        origin: NSPoint { x: glass.origin.x - SHADOW_MARGIN, y: glass.origin.y - SHADOW_MARGIN },
        size: NSSize {
            width: glass.size.width + 2.0 * SHADOW_MARGIN,
            height: glass.size.height + SHADOW_MARGIN + room_above,
        },
    }
}

fn at_origin(size: NSSize) -> NSRect {
    NSRect { origin: NSPoint { x: 0.0, y: 0.0 }, size }
}
