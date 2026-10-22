//! The Permissions window: one row per permission with its status and remedy, a line naming
//! the app macOS attributes the permissions to, and Quit and Continue. It opens when the
//! runtime waits for permissions, and from the menu at any time.

pub mod mapping;

use std::cell::RefCell;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationDidBecomeActiveNotification, NSBackingStoreType, NSButton, NSColor,
    NSFont, NSFontWeightMedium, NSImage, NSImageSymbolConfiguration, NSImageView,
    NSProgressIndicator, NSProgressIndicatorStyle, NSTextAlignment, NSTextField, NSView, NSWindow,
    NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{
    NSNotificationCenter, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSURL,
};

use crate::adapters::main_queue::{dispatch_to_main, dispatch_to_main_after};
use crate::core::application::PermissionsGate;
use crate::core::domain::Permission;
use crate::runtime::{RuntimeObserver, RuntimeState};

use mapping::{badge, button, symbol, why, BadgeMark, RowButton, Shown, WindowModel};

const WIDTH: f64 = 460.0;
const HEIGHT: f64 = 390.0;
const MARGIN: f64 = 20.0;
const ROW_HEIGHT: f64 = 44.0;
const ROW_GAP: f64 = 12.0;
const TILE: f64 = 36.0;
const SYMBOL_SIZE: f64 = 22.0;
const STATUS_WIDTH: f64 = 140.0;
const FOOTER_BUTTON_WIDTH: f64 = 90.0;
/// How often the open window asks the gate again.
const REFRESH_MS: i64 = 2_000;
/// `NSFloatingWindowLevel`: above ordinary windows, so System Settings does not hide it.
const FLOATING_LEVEL: isize = 3;
const QUIT_TITLE: &str = "Quit";
const CONTINUE_TITLE: &str = "Continue";

thread_local! {
    /// The window, reached by the work dispatched to the main queue.
    static WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
}

struct Window {
    window: Retained<NSWindow>,
    target: Retained<WindowTarget>,
    gate: Arc<PermissionsGate>,
    /// Requested and not answered yet.
    pending: Vec<Permission>,
    /// Revoked while dictation ran, as the runtime last said.
    revoked: Vec<Permission>,
    /// Focus is taken only the first time the window shows.
    shown_once: bool,
    /// Bumped by every show; a refresh loop of an older show stops.
    showing: u64,
    quit: Option<Box<dyn Fn()>>,
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
        WINDOW.set(Some(Window {
            window,
            target,
            gate,
            pending: Vec::new(),
            revoked: Vec::new(),
            shown_once: false,
            showing: 0,
            quit: None,
        }));
        Self
    }

    /// What Quit does.
    pub fn set_quit(&self, quit: Box<dyn Fn()>) {
        WINDOW.with_borrow_mut(|state| {
            if let Some(state) = state {
                state.quit = Some(quit);
            }
        });
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
        let model = WindowModel::new(&report, &state.pending, &state.revoked);
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
    let url = WINDOW.with_borrow(|state| state.as_ref().map(|s| s.gate.settings_url(permission)));
    let Some(url) = url else { return };
    let opened = NSURL::URLWithString(&NSString::from_str(&url))
        .is_some_and(|url| NSWorkspace::sharedWorkspace().openURL(&url));
    if !opened {
        log::warn!("permissions: System Settings not opened at {url}");
    }
}

fn quit() {
    let quit = WINDOW.with_borrow_mut(|state| state.as_mut().and_then(|s| s.quit.take()));
    match quit {
        Some(quit) => quit(),
        None => std::process::exit(0),
    }
}

fn close() {
    WINDOW.with_borrow(|state| {
        if let Some(state) = state {
            state.window.orderOut(None);
        }
    });
}

fn build_window(mtm: MainThreadMarker) -> Retained<NSWindow> {
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, HEIGHT));
    // SAFETY: a new titled window; it is not released on close, the state keeps it.
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSWindow>(),
            frame,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    // SAFETY: a Retained owns the window, so AppKit must not release it again on close.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&NSString::from_str(mapping::TITLE));
    window.setLevel(FLOATING_LEVEL);
    window
}

/// Frame `height` high, `y` points below the content's top edge.
fn below_top(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(x, HEIGHT - y - height),
        NSSize::new(width, height),
    )
}

#[expect(clippy::too_many_lines, reason = "size baseline")]
fn content(mtm: MainThreadMarker, target: &WindowTarget, model: &WindowModel) -> Retained<NSView> {
    let view = NSView::initWithFrame(
        mtm.alloc::<NSView>(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, HEIGHT)),
    );
    let text_width = WIDTH - 2.0 * MARGIN;
    let heading = label(mtm, mapping::HEADING, NSFont::boldSystemFontOfSize(15.0));
    heading.setFrame(below_top(MARGIN, 14.0, text_width, 20.0));
    view.addSubview(&heading);
    let sub = wrapping(mtm, mapping::SUBHEADING, NSFont::systemFontOfSize(13.0));
    sub.setTextColor(Some(&NSColor::secondaryLabelColor()));
    sub.setFrame(below_top(MARGIN, 36.0, text_width, 34.0));
    view.addSubview(&sub);
    let mut rows_top = 80.0;
    let note = model
        .revoked_line
        .as_deref()
        .map(|line| (line, NSColor::systemRedColor()))
        .or_else(|| {
            model
                .developer_line
                .as_deref()
                .map(|line| (line, NSColor::systemOrangeColor()))
        });
    if let Some((line, colour)) = note {
        let note = label(mtm, line, medium(13.0));
        note.setTextColor(Some(&colour));
        note.setFrame(below_top(MARGIN, 74.0, text_width, 18.0));
        view.addSubview(&note);
        rows_top = 100.0;
    }
    for (index, row) in model.rows.iter().enumerate() {
        let top = rows_top + index as f64 * (ROW_HEIGHT + ROW_GAP);
        add_row(mtm, &view, target, row.permission, row.shown, top);
    }
    let quit = push_button(mtm, target, QUIT_TITLE, sel!(quit:), 0);
    quit.setFrame(below_top(MARGIN, HEIGHT - 42.0, FOOTER_BUTTON_WIDTH, 28.0));
    view.addSubview(&quit);
    let go_on = push_button(mtm, target, CONTINUE_TITLE, sel!(goOn:), 0);
    go_on.setKeyEquivalent(&NSString::from_str("\r"));
    go_on.setEnabled(model.continue_enabled);
    go_on.setFrame(below_top(
        WIDTH - MARGIN - FOOTER_BUTTON_WIDTH,
        HEIGHT - 42.0,
        FOOTER_BUTTON_WIDTH,
        28.0,
    ));
    view.addSubview(&go_on);
    view
}

#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "size baseline"
)]
fn add_row(
    mtm: MainThreadMarker,
    view: &NSView,
    target: &WindowTarget,
    permission: Permission,
    shown: Shown,
    top: f64,
) {
    let tile = NSView::initWithFrame(
        mtm.alloc::<NSView>(),
        below_top(MARGIN, top + (ROW_HEIGHT - TILE) / 2.0, TILE, TILE),
    );
    tile.setWantsLayer(true);
    if let Some(layer) = tile.layer() {
        let tint = NSColor::systemBlueColor().colorWithAlphaComponent(0.15);
        layer.setBackgroundColor(Some(&tint.CGColor()));
        layer.setCornerRadius(8.0);
    }
    if let Some(image) = symbol_image(symbol(permission)) {
        let picture = NSImageView::imageViewWithImage(&image, mtm);
        picture.setContentTintColor(Some(&NSColor::systemBlueColor()));
        picture.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(TILE, TILE)));
        tile.addSubview(&picture);
    }
    view.addSubview(&tile);
    let text_x = MARGIN + TILE + 10.0;
    let text_width = WIDTH - MARGIN - STATUS_WIDTH - 10.0 - text_x;
    let name = label(mtm, permission.name(), medium(13.0));
    name.setFrame(below_top(text_x, top + 2.0, text_width, 17.0));
    view.addSubview(&name);
    let reason = wrapping(mtm, why(permission), NSFont::systemFontOfSize(12.0));
    reason.setTextColor(Some(&NSColor::secondaryLabelColor()));
    reason.setFrame(below_top(text_x, top + 19.0, text_width, 30.0));
    view.addSubview(&reason);
    add_badge(mtm, view, shown, top);
    if let Some(choice) = button(shown) {
        let action = match choice {
            RowButton::Request => sel!(request:),
            RowButton::OpenSystemSettings => sel!(openSettings:),
        };
        let index = Permission::ALL
            .iter()
            .position(|p| *p == permission)
            .unwrap_or_default();
        let control = push_button(mtm, target, choice.title(), action, index as isize);
        control.setFrame(below_top(
            WIDTH - MARGIN - STATUS_WIDTH,
            top + 18.0,
            STATUS_WIDTH,
            24.0,
        ));
        view.addSubview(&control);
    }
}

/// The badge at the right of a row: a coloured dot or a spinner, then the words.
fn add_badge(mtm: MainThreadMarker, view: &NSView, shown: Shown, top: f64) {
    let (words, mark) = badge(shown);
    let right = WIDTH - MARGIN;
    let text = label(mtm, words, NSFont::systemFontOfSize(11.0));
    text.setTextColor(Some(&NSColor::secondaryLabelColor()));
    text.sizeToFit();
    let width = text.frame().size.width;
    let y = if button(shown).is_some() {
        top
    } else {
        top + 13.0
    };
    text.setFrame(below_top(right - width, y, width, 16.0));
    view.addSubview(&text);
    let mark_x = right - width - 14.0;
    match mark {
        BadgeMark::Spinner => {
            let spinner = NSProgressIndicator::initWithFrame(
                mtm.alloc::<NSProgressIndicator>(),
                below_top(mark_x, y + 2.0, 12.0, 12.0),
            );
            spinner.setStyle(NSProgressIndicatorStyle::Spinning);
            spinner.setIndeterminate(true);
            // SAFETY: a main-thread view built with a MainThreadMarker; a nil sender is allowed.
            unsafe { spinner.startAnimation(None) };
            view.addSubview(&spinner);
        }
        mark => {
            let colour = match mark {
                BadgeMark::Green => NSColor::systemGreenColor(),
                BadgeMark::Red => NSColor::systemRedColor(),
                _ => NSColor::systemGrayColor(),
            };
            let dot = label(mtm, "●", NSFont::systemFontOfSize(9.0));
            dot.setTextColor(Some(&colour));
            dot.setFrame(below_top(mark_x, y + 1.0, 12.0, 14.0));
            view.addSubview(&dot);
        }
    }
}

fn medium(size: f64) -> Retained<NSFont> {
    // SAFETY: an immutable framework constant, set before any Rust code runs.
    NSFont::systemFontOfSize_weight(size, unsafe { NSFontWeightMedium })
}

fn label(mtm: MainThreadMarker, text: &str, font: Retained<NSFont>) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    field.setFont(Some(&font));
    field.setAlignment(NSTextAlignment::Left);
    field
}

fn wrapping(mtm: MainThreadMarker, text: &str, font: Retained<NSFont>) -> Retained<NSTextField> {
    let field = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
    field.setFont(Some(&font));
    field
}

fn push_button(
    mtm: MainThreadMarker,
    target: &WindowTarget,
    title: &str,
    action: Sel,
    tag: isize,
) -> Retained<NSButton> {
    // SAFETY: `action` is a method of `target` taking the sender, as a button calls it.
    let control = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target as &AnyObject),
            Some(action),
            mtm,
        )
    };
    control.setTag(tag);
    control
}

fn symbol_image(name: &str) -> Option<Retained<NSImage>> {
    let name = NSString::from_str(name);
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&name, Some(&name))?;
    let config =
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        NSImageSymbolConfiguration::configurationWithPointSize_weight(SYMBOL_SIZE, unsafe {
            objc2_app_kit::NSFontWeightRegular
        });
    image.imageWithSymbolConfiguration(&config)
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
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: `NSObject`'s `init` on a freshly allocated object.
        unsafe { msg_send![super(this), init] }
    }
}
