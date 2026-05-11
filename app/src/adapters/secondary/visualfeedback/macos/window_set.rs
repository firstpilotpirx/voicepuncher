use std::sync::Arc;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSControlSize, NSFont, NSProgressIndicator,
    NSProgressIndicatorStyle, NSTextAlignment, NSTextField, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::kCACornerCurveContinuous;

pub(super) struct WindowSet {
    pub(super) window: Retained<NSWindow>,
    pub(super) label: Retained<NSTextField>,
    pub(super) spinner: Retained<NSProgressIndicator>,
}

impl WindowSet {
    pub(super) fn build(mtm: MainThreadMarker, visible_frame: NSRect) -> Arc<Self> {
        let window_width: f64 = 140.0;
        let window_height: f64 = 55.0;

        let x = visible_frame.origin.x + (visible_frame.size.width - window_width) / 2.0;
        let y = visible_frame.origin.y + visible_frame.size.height - window_height - 4.0;

        let frame = NSRect {
            origin: NSPoint { x, y },
            size: NSSize {
                width: window_width,
                height: window_height,
            },
        };

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc::<NSWindow>(),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };

        window.setLevel(3);
        window.setOpaque(false);
        window.setBackgroundColor(Some(&NSColor::clearColor()));
        window.setIgnoresMouseEvents(true);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );

        let content_frame = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize {
                width: window_width,
                height: window_height,
            },
        };
        let effect_view =
            NSVisualEffectView::initWithFrame(mtm.alloc::<NSVisualEffectView>(), content_frame);
        effect_view.setMaterial(NSVisualEffectMaterial::HUDWindow);
        effect_view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect_view.setState(NSVisualEffectState::Active);
        effect_view.setWantsLayer(true);
        if let Some(layer) = effect_view.layer() {
            let continuous_curve = unsafe { kCACornerCurveContinuous };
            layer.setCornerCurve(continuous_curve);
            layer.setCornerRadius(20.0);
            layer.setMasksToBounds(true);
            let dark_fill = NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 0.0, 0.28);
            layer.setBackgroundColor(Some(&dark_fill.CGColor()));
            let border = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.25);
            layer.setBorderColor(Some(&border.CGColor()));
            layer.setBorderWidth(0.5);
        }
        window.setContentView(Some(&*effect_view));

        let font = NSFont::boldSystemFontOfSize(28.0);
        let label_height = (font.ascender() - font.descender()).ceil() + 4.0;
        let label_y =
            (window_height / 2.0 - font.capHeight() / 2.0 - (label_height - font.ascender()))
                .floor();
        let label_frame = NSRect {
            origin: NSPoint {
                x: 12.0,
                y: label_y,
            },
            size: NSSize {
                width: window_width - 24.0,
                height: label_height,
            },
        };

        let label = NSTextField::initWithFrame(mtm.alloc::<NSTextField>(), label_frame);
        label.setBezeled(false);
        label.setEditable(false);
        label.setDrawsBackground(false);
        label.setAlignment(NSTextAlignment::Center);
        label.setFont(Some(&font));
        label.setTextColor(Some(&NSColor::whiteColor()));
        let empty = NSString::from_str("");
        label.setStringValue(&empty);

        effect_view.addSubview(&label);

        let spinner_size: f64 = 32.0;
        let spinner_x = (window_width - spinner_size) / 2.0;
        let spinner_frame = NSRect {
            origin: NSPoint {
                x: spinner_x,
                y: (window_height - spinner_size) / 2.0,
            },
            size: NSSize {
                width: spinner_size,
                height: spinner_size,
            },
        };

        let spinner =
            NSProgressIndicator::initWithFrame(mtm.alloc::<NSProgressIndicator>(), spinner_frame);
        spinner.setStyle(NSProgressIndicatorStyle::Spinning);
        spinner.setIndeterminate(true);
        spinner.setControlSize(NSControlSize::Regular);
        spinner.setHidden(true);

        effect_view.addSubview(&spinner);

        Arc::new(Self {
            window,
            label,
            spinner,
        })
    }
}

// Safety: WindowSet is accessed exclusively on the main thread. All reads and
// writes to the contained Retained<…> objects go through dispatch_async_f /
// dispatch_after_f targeting the main queue, or directly inside functions that
// require MainThreadMarker. The debug_assert in Drop enforces this at runtime
// in debug builds.
unsafe impl Send for WindowSet {}
unsafe impl Sync for WindowSet {}

impl Drop for WindowSet {
    fn drop(&mut self) {
        debug_assert!(
            MainThreadMarker::new().is_some(),
            "WindowSet must be dropped on the main thread"
        );
        self.window.orderOut(None);
    }
}
