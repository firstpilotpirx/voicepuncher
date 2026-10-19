//! The dictation sign's badge: a round dark-glass disc with an SF Symbol per stage of the
//! dictation, and an activity ring that turns while a local service works on the text.

use std::f64::consts::TAU;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSColor, NSImage,
    NSImageView, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectView,
};
use objc2_core_graphics::CGPath;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::{kCALineCapRound, kCAMediaTimingFunctionLinear, CAShapeLayer};

use super::motion::{endless_animation, reduce_motion, set_breathing, timing};
use super::symbols::symbol_image;
use crate::adapters::secondary::visualfeedback::pointer_sign::DictationSignState;

/// The badge's diameter.
pub(super) const BADGE_SIZE: f64 = 38.0;
const RIM_WIDTH: f64 = 1.0;
const RIM_WHITE_ALPHA: f64 = 0.55;
const ICON_POINT_SIZE: f64 = 19.0;
/// The square the icon is drawn in, centred in the badge.
const ICON_FRAME: f64 = 26.0;
const RING_WIDTH: f64 = 2.0;
/// How far inside the badge's edge the activity ring runs.
const RING_INSET: f64 = 2.0;
const RING_WHITE_ALPHA: f64 = 0.9;
/// The share of a full circle the activity ring covers: 270°.
const RING_ARC: f64 = 0.75;
const RING_TURN_SECS: f64 = 0.9;
const RING_ANIMATION: &str = "turn";

/// The SF Symbol each badge shows.
pub(super) fn badge_symbol(badge: DictationSignState) -> &'static str {
    match badge {
        DictationSignState::Recording => "mic.fill",
        DictationSignState::RecognizingSpeech => "ear",
        DictationSignState::RefiningText => "sparkles",
        DictationSignState::WaitingForFocus => "keyboard",
        DictationSignState::Pasted => "checkmark",
        DictationSignState::Failure => "exclamationmark.triangle.fill",
    }
}

fn badge_tint(badge: DictationSignState) -> Retained<NSColor> {
    match badge {
        DictationSignState::Recording => NSColor::systemRedColor(),
        DictationSignState::RecognizingSpeech | DictationSignState::RefiningText => {
            NSColor::whiteColor()
        }
        DictationSignState::WaitingForFocus => NSColor::systemOrangeColor(),
        DictationSignState::Pasted => NSColor::systemGreenColor(),
        DictationSignState::Failure => NSColor::systemYellowColor(),
    }
}

/// The badge's views; touched on the main thread only.
pub(super) struct BadgeView {
    glass: Retained<NSVisualEffectView>,
    icon: Retained<NSImageView>,
    ring: Retained<CAShapeLayer>,
}

impl BadgeView {
    pub(super) fn build(mtm: MainThreadMarker) -> Self {
        let glass = build_glass(mtm);
        let ring_host = NSView::initWithFrame(mtm.alloc::<NSView>(), badge_rect());
        ring_host.setWantsLayer(true);
        let ring = build_ring();
        if let Some(layer) = ring_host.layer() {
            layer.addSublayer(&ring);
        }
        glass.addSubview(&ring_host);
        let icon = NSImageView::initWithFrame(mtm.alloc::<NSImageView>(), icon_rect());
        icon.setWantsLayer(true);
        glass.addSubview(&icon);
        Self { glass, icon, ring }
    }

    /// The view that goes into the sign's window.
    pub(super) fn view(&self) -> &NSView {
        &self.glass
    }

    /// Shows `badge`: its icon and tint, the turning ring while processing, and breathing
    /// unless the user asked macOS to reduce motion.
    pub(super) fn apply(&self, badge: DictationSignState) {
        self.icon.setImage(badge_image(badge).as_deref());
        self.icon.setContentTintColor(Some(&badge_tint(badge)));
        self.set_ring_turning(badge.turns_activity_ring());
        self.set_breathing(badge.breathes() && !reduce_motion());
    }

    /// Stops every animation of a hidden badge.
    pub(super) fn stop(&self) {
        self.set_ring_turning(false);
        self.set_breathing(false);
    }

    pub(super) fn shows_icon(&self) -> bool {
        self.icon.image().is_some()
    }

    pub(super) fn activity_ring_turns(&self) -> bool {
        let key = NSString::from_str(RING_ANIMATION);
        // SAFETY: the animation is only checked for presence, never changed.
        let turn = unsafe { self.ring.animationForKey(&key) };
        !self.ring.isHidden() && turn.is_some()
    }

    fn set_ring_turning(&self, turning: bool) {
        self.ring.removeAllAnimations();
        self.ring.setHidden(!turning);
        if turning {
            let turn = endless_animation("transform.rotation.z", 0.0, -TAU, RING_TURN_SECS);
            turn.setTimingFunction(Some(&timing(unsafe { kCAMediaTimingFunctionLinear })));
            self.ring
                .addAnimation_forKey(&turn, Some(&NSString::from_str(RING_ANIMATION)));
        }
    }

    fn set_breathing(&self, breathing: bool) {
        if let Some(layer) = self.icon.layer() {
            set_breathing(&layer, breathing);
        }
    }
}

fn badge_image(badge: DictationSignState) -> Option<Retained<NSImage>> {
    symbol_image(badge_symbol(badge), ICON_POINT_SIZE)
}

fn build_glass(mtm: MainThreadMarker) -> Retained<NSVisualEffectView> {
    let glass = NSVisualEffectView::initWithFrame(mtm.alloc::<NSVisualEffectView>(), badge_rect());
    glass.setMaterial(NSVisualEffectMaterial::HUDWindow);
    glass.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    glass.setState(NSVisualEffectState::Active);
    if let Some(dark) = NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }) {
        glass.setAppearance(Some(&dark));
    }
    glass.setWantsLayer(true);
    if let Some(layer) = glass.layer() {
        layer.setCornerRadius(BADGE_SIZE / 2.0);
        layer.setMasksToBounds(true);
        let rim = NSColor::colorWithWhite_alpha(1.0, RIM_WHITE_ALPHA);
        layer.setBorderColor(Some(&rim.CGColor()));
        layer.setBorderWidth(RIM_WIDTH);
    }
    glass
}

fn build_ring() -> Retained<CAShapeLayer> {
    let ring = CAShapeLayer::new();
    ring.setFrame(badge_rect());
    let inset = RING_INSET + RING_WIDTH / 2.0;
    let circle = NSRect {
        origin: NSPoint { x: inset, y: inset },
        size: NSSize {
            width: BADGE_SIZE - 2.0 * inset,
            height: BADGE_SIZE - 2.0 * inset,
        },
    };
    // SAFETY: a null transform is documented as "no transform".
    let path = unsafe { CGPath::with_ellipse_in_rect(circle, std::ptr::null()) };
    ring.setPath(Some(&path));
    ring.setFillColor(None);
    let stroke = NSColor::colorWithWhite_alpha(1.0, RING_WHITE_ALPHA);
    ring.setStrokeColor(Some(&stroke.CGColor()));
    ring.setLineWidth(RING_WIDTH);
    ring.setLineCap(unsafe { kCALineCapRound });
    ring.setStrokeEnd(RING_ARC);
    ring.setHidden(true);
    ring
}

fn badge_rect() -> NSRect {
    NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: BADGE_SIZE,
            height: BADGE_SIZE,
        },
    }
}

fn icon_rect() -> NSRect {
    let offset = (BADGE_SIZE - ICON_FRAME) / 2.0;
    NSRect {
        origin: NSPoint {
            x: offset,
            y: offset,
        },
        size: NSSize {
            width: ICON_FRAME,
            height: ICON_FRAME,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_show_the_symbol_of_each_stage() {
        let expected = [
            (DictationSignState::Recording, "mic.fill"),
            (DictationSignState::RecognizingSpeech, "ear"),
            (DictationSignState::RefiningText, "sparkles"),
            (DictationSignState::WaitingForFocus, "keyboard"),
            (DictationSignState::Pasted, "checkmark"),
            (DictationSignState::Failure, "exclamationmark.triangle.fill"),
        ];
        for (badge, symbol) in expected {
            assert_eq!(badge_symbol(badge), symbol, "{badge:?}");
        }
    }
}
