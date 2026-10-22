//! The decorative layer animations of the overlay and the pointer badge, and the Reduce Motion
//! setting that stops them.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSNumber, NSString};
use objc2_quartz_core::{
    kCAMediaTimingFunctionEaseInEaseOut, CABasicAnimation, CALayer, CAMediaTiming,
    CAMediaTimingFunction, CAMediaTimingFunctionName,
};

/// The icon breathes from full opacity to this and back.
const BREATH_LOW_OPACITY: f64 = 0.4;
/// Half a breath: 1.4 s in and out.
const BREATH_HALF_SECS: f64 = 0.7;
const BREATH_ANIMATION: &str = "breathe";
/// The record dot shrinks to this scale and back.
const PULSE_LOW_SCALE: f64 = 0.8;
/// The record dot fades to this opacity and back.
const PULSE_LOW_OPACITY: f64 = 0.75;
/// Half a pulse: 1.6 s in and out.
const PULSE_HALF_SECS: f64 = 0.8;
const PULSE_SCALE_ANIMATION: &str = "pulse-scale";
const PULSE_OPACITY_ANIMATION: &str = "pulse-opacity";

/// Whether the user asked macOS to reduce motion; read each time a state is drawn.
pub(super) fn reduce_motion() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

/// Makes `layer` breathe forever, or stops it.
pub(super) fn set_breathing(layer: &CALayer, breathing: bool) {
    remove(layer, BREATH_ANIMATION);
    if breathing {
        let breath = there_and_back("opacity", BREATH_LOW_OPACITY, BREATH_HALF_SECS);
        layer.addAnimation_forKey(&breath, Some(&NSString::from_str(BREATH_ANIMATION)));
    }
}

/// Makes the record dot pulse forever, or stops it.
pub(super) fn set_pulsing(layer: &CALayer, pulsing: bool) {
    remove(layer, PULSE_SCALE_ANIMATION);
    remove(layer, PULSE_OPACITY_ANIMATION);
    if pulsing {
        let scale = there_and_back("transform.scale", PULSE_LOW_SCALE, PULSE_HALF_SECS);
        let fade = there_and_back("opacity", PULSE_LOW_OPACITY, PULSE_HALF_SECS);
        layer.addAnimation_forKey(&scale, Some(&NSString::from_str(PULSE_SCALE_ANIMATION)));
        layer.addAnimation_forKey(&fade, Some(&NSString::from_str(PULSE_OPACITY_ANIMATION)));
    }
}

/// Whether `layer` breathes now.
pub(super) fn is_breathing(layer: &CALayer) -> bool {
    has(layer, BREATH_ANIMATION)
}

/// Whether `layer` pulses now.
pub(super) fn is_pulsing(layer: &CALayer) -> bool {
    has(layer, PULSE_SCALE_ANIMATION)
}

fn has(layer: &CALayer, key: &str) -> bool {
    // SAFETY: the animation is only checked for presence, never changed.
    unsafe { layer.animationForKey(&NSString::from_str(key)) }.is_some()
}

fn remove(layer: &CALayer, key: &str) {
    layer.removeAnimationForKey(&NSString::from_str(key));
}

/// From 1 to `low` and back, ease-in-out, `half_secs` each way, forever.
fn there_and_back(key_path: &str, low: f64, half_secs: f64) -> Retained<CABasicAnimation> {
    let animation = endless_animation(key_path, 1.0, low, half_secs);
    animation.setAutoreverses(true);
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    animation.setTimingFunction(Some(&timing(unsafe {
        kCAMediaTimingFunctionEaseInEaseOut
    })));
    animation
}

pub(super) fn timing(name: &CAMediaTimingFunctionName) -> Retained<CAMediaTimingFunction> {
    CAMediaTimingFunction::functionWithName(name)
}

/// An animation of `key_path` from `from` to `to` over `secs`, repeated forever.
pub(super) fn endless_animation(
    key_path: &str,
    from: f64,
    to: f64,
    secs: f64,
) -> Retained<CABasicAnimation> {
    let animation = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str(key_path)));
    let from = NSNumber::new_f64(from);
    let to = NSNumber::new_f64(to);
    let from: &AnyObject = &from;
    let to: &AnyObject = &to;
    // SAFETY: both values are NSNumbers, the type the animated layer property takes.
    unsafe { animation.setFromValue(Some(from)) };
    // SAFETY: as above.
    unsafe { animation.setToValue(Some(to)) };
    animation.setDuration(secs);
    animation.setRepeatCount(f32::INFINITY);
    animation
}
