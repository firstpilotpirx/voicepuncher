//! The Permissions window: one row per permission with its status and remedy, a line naming
//! the app macOS attributes the permissions to, and Quit and Continue.
//!
//! It opens when the runtime waits for permissions, and from the menu at any time.

pub mod mapping;
mod system_settings;

use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationDidBecomeActiveNotification, NSButton, NSColor, NSFont,
    NSFontWeightMedium, NSImage, NSImageSymbolConfiguration, NSImageView, NSProgressIndicator,
    NSProgressIndicatorStyle, NSView, NSWindow,
};
use objc2_foundation::{
    NSNotificationCenter, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
};

use crate::adapters::appkit::{self, label, wrapping};
use crate::adapters::main_queue::{dispatch_to_main, dispatch_to_main_after};
use crate::core::application::PermissionsGate;
use crate::core::domain::Permission;
use crate::runtime::{RuntimeObserver, RuntimeState};

use mapping::{
    badge, button, symbol, BadgeMark, RowButton, RowModel, Shown, WindowModel, REFRESH_MS,
};

mod view;
use view::content;

const WIDTH: f64 = 460.0;
const HEIGHT: f64 = 390.0;
const MARGIN: f64 = 20.0;
const ROW_HEIGHT: f64 = 44.0;
const ROW_GAP: f64 = 12.0;
const TILE: f64 = 36.0;
const SYMBOL_SIZE: f64 = 22.0;
const STATUS_WIDTH: f64 = 140.0;
const FOOTER_BUTTON_WIDTH: f64 = 90.0;
/// `NSFloatingWindowLevel`: above ordinary windows, so System Settings does not hide it.
const FLOATING_LEVEL: isize = 3;
const QUIT_TITLE: &str = "Quit";
const CONTINUE_TITLE: &str = "Continue";

appkit::kept_window_slot!(ShownWindow);

struct ShownWindow {
    window: Retained<NSWindow>,
    target: Retained<WindowTarget>,
    gate: Arc<PermissionsGate>,
    /// Requested and not answered yet.
    pending: Vec<Permission>,
    /// System Settings was opened from their rows.
    opened: Vec<Permission>,
    /// Revoked while dictation ran, as the runtime last said.
    revoked: Vec<Permission>,
    /// Focus is taken only the first time the window shows.
    shown_once: bool,
    /// Bumped by every show; a refresh loop of an older show stops.
    showing: u64,
    quit: Option<appkit::QuitAction>,
}

/// The Permissions window of the app.
pub struct PermissionsWindow;

impl PermissionsWindow {
    /// Builds the window, hidden until the runtime waits for permissions or the menu asks.
    pub fn install(mtm: MainThreadMarker, gate: Arc<PermissionsGate>) -> Self {
        let window = build_window(mtm);
        let target = WindowTarget::new(mtm);
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        let active = unsafe { NSApplicationDidBecomeActiveNotification };
        let (center, selector) = (
            NSNotificationCenter::defaultCenter(),
            sel!(appBecameActive:),
        );
        // SAFETY: the target lives as long as the window state that holds both.
        unsafe { center.addObserver_selector_name_object(&target, selector, Some(active), None) };
        WINDOW.set(Some(ShownWindow {
            window,
            target,
            gate,
            pending: Vec::new(),
            opened: Vec::new(),
            revoked: Vec::new(),
            shown_once: false,
            showing: 0,
            quit: None,
        }));
        Self
    }

    /// What Quit does.
    pub fn set_quit(&self, quit: appkit::QuitAction) {
        appkit::set_quit(&WINDOW, quit);
    }

    /// Opens the window on Waiting for permissions and keeps it current on other states.
    pub fn observer(&self) -> PermissionsWindowObserver {
        PermissionsWindowObserver
    }
}

/// Brings the window to the front, from the menu.
pub fn bring_to_front() {
    dispatch_to_main(|| show(None, true));
}

/// Tells the window each runtime state.
pub struct PermissionsWindowObserver;

impl RuntimeObserver for PermissionsWindowObserver {
    fn state_changed(&self, state: &RuntimeState) {
        match state {
            RuntimeState::WaitingForPermissions { revoked, .. } => {
                let revoked = revoked.clone();
                dispatch_to_main(move || show(Some(revoked), false));
            }
            RuntimeState::Ready { .. } => dispatch_to_main(|| {
                set_revoked(Vec::new());
                refresh();
            }),
            _ => {}
        }
    }
}

fn set_revoked(revoked: Vec<Permission>) {
    WINDOW.with_borrow_mut(|state| {
        if let Some(state) = state {
            state.revoked = revoked;
        }
    });
}

/// Shows the window; `revoked` replaces what it says was revoked, `to_front` takes focus.
fn show(revoked: Option<Vec<Permission>>, to_front: bool) {
    if let Some(revoked) = revoked {
        set_revoked(revoked);
    }
    refresh();
    let showing = WINDOW.with_borrow_mut(|state| {
        let state = state.as_mut()?;
        let visible = state.window.isVisible();
        if to_front || !state.shown_once {
            if !state.shown_once {
                state.window.center();
            }
            state.shown_once = true;
            let mtm = MainThreadMarker::from(&*state.target);
            NSApplication::sharedApplication(mtm).activate();
            state.window.makeKeyAndOrderFront(None);
        } else if !visible {
            state.window.orderFrontRegardless();
        }
        if visible {
            return None;
        }
        state.showing += 1;
        Some(state.showing)
    });
    if let Some(showing) = showing {
        keep_current(showing);
    }
}

/// Asks the gate again every [`REFRESH_MS`] while the window shows.
fn keep_current(showing: u64) {
    dispatch_to_main_after(REFRESH_MS, move || {
        let current = WINDOW.with_borrow(|state| {
            state
                .as_ref()
                .is_some_and(|s| s.showing == showing && s.window.isVisible())
        });
        if current {
            refresh();
            keep_current(showing);
        }
    });
}

/// Draws the window from the gate's report at this moment.
fn refresh() {
    WINDOW.with_borrow_mut(|state| {
        let Some(state) = state else { return };
        let report = state.gate.report();
        state
            .pending
            .retain(|p| report.status(*p) == crate::core::domain::PermissionStatus::NotDetermined);
        let model = WindowModel::new(&report, &state.pending, &state.revoked)
            .with_lines(&state.opened, system_settings::macos_major());
        let mtm = MainThreadMarker::from(&*state.target);
        state
            .window
            .setContentView(Some(&content(mtm, &state.target, &model)));
    });
}

fn request(permission: Permission) {
    let gate = WINDOW.with_borrow_mut(|state| {
        let state = state.as_mut()?;
        if !state.pending.contains(&permission) {
            state.pending.push(permission);
        }
        Some(Arc::clone(&state.gate))
    });
    // not now: the button that asked is part of the content a refresh replaces
    dispatch_to_main(refresh);
    if let Some(gate) = gate {
        // a request may wait for the user's answer to the system prompt
        let spawned = std::thread::Builder::new()
            .name("permission-request".into())
            .spawn(move || {
                gate.request(permission);
                dispatch_to_main(refresh);
            });
        if let Err(e) = spawned {
            log::warn!("permissions: {permission} not requested: {e}");
        }
    }
}

fn open_settings(permission: Permission) {
    let gate = WINDOW.with_borrow_mut(|state| {
        let state = state.as_mut()?;
        if !state.opened.contains(&permission) {
            state.opened.push(permission);
        }
        Some(Arc::clone(&state.gate))
    });
    if let Some(gate) = gate {
        system_settings::open(&gate, permission);
    }
    dispatch_to_main(refresh);
}

fn quit() {
    appkit::quit(&WINDOW);
}

fn close() {
    appkit::close(&WINDOW);
}

fn build_window(mtm: MainThreadMarker) -> Retained<NSWindow> {
    let window = appkit::titled_window(mtm, mapping::TITLE, WIDTH, HEIGHT);
    window.setLevel(FLOATING_LEVEL);
    window
}

/// Frame `height` high, `y` points below the content's top edge.
fn below_top(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    appkit::below_top(HEIGHT, x, y, width, height)
}

/// The permission of the row whose button sent the action.
fn sender_permission(sender: Option<&AnyObject>) -> Option<Permission> {
    let sender = sender?.downcast_ref::<NSButton>()?;
    Permission::ALL
        .get(usize::try_from(sender.tag()).ok()?)
        .copied()
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPPermissionsWindowTarget"]
    struct WindowTarget;

    unsafe impl NSObjectProtocol for WindowTarget {}

    impl WindowTarget {
        #[unsafe(method(request:))]
        fn request(&self, sender: Option<&AnyObject>) {
            if let Some(permission) = sender_permission(sender) {
                request(permission);
            }
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, sender: Option<&AnyObject>) {
            if let Some(permission) = sender_permission(sender) {
                open_settings(permission);
            }
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            quit();
        }

        #[unsafe(method(goOn:))]
        fn go_on(&self, _sender: Option<&AnyObject>) {
            close();
        }

        #[unsafe(method(appBecameActive:))]
        fn app_became_active(&self, _notification: Option<&AnyObject>) {
            // the target is borrowed by no one here: AppKit calls it from the run loop
            dispatch_to_main(refresh);
        }
    }
);

impl WindowTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        appkit::new_plain_object(mtm)
    }
}
