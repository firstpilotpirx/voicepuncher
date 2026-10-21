//! The Download window: one row per model file the configuration needs with its purpose, size
//! and status, one bar for the total, the free space, and Open Folder, Quit, Download (Cancel
//! while it runs, Retry after a failure) and Done. It opens when the runtime waits for models,
//! and from the menu and the Services tab at any time; the download runs on a thread of its
//! own and every event is drawn on the main thread.

pub mod mapping;

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSButton, NSColor, NSFont, NSFontWeightMedium,
    NSProgressIndicator, NSProgressIndicatorStyle, NSTextAlignment, NSTextField, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use crate::adapters::main_queue::dispatch_to_main;
use crate::adapters::primary::menubar::macos::actions::show_in_file_browser;
use crate::core::application::{DownloadEvent, ModelDownload};
use crate::runtime::{RuntimeObserver, RuntimeState};

use mapping::{human_size, status_text, MainButton, WindowModel};

const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 440.0;
const MARGIN: f64 = 20.0;
const ROW_HEIGHT: f64 = 38.0;
const TABLE_TOP: f64 = 70.0;
const SIZE_X: f64 = 330.0;
const SIZE_WIDTH: f64 = 70.0;
const STATUS_X: f64 = 405.0;
const BUTTON_WIDTH: f64 = 100.0;
/// Progress is drawn at most this often; every other event at once.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// Builds the download of the files the current configuration needs, with its models folder.
pub type DownloadSource = Box<dyn Fn() -> (ModelDownload, PathBuf)>;

thread_local! {
    /// The window, reached by the work dispatched to the main queue.
    static WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
}

struct Window {
    window: Retained<NSWindow>,
    target: Retained<WindowTarget>,
    source: DownloadSource,
    model: Option<WindowModel>,
    /// Set by Cancel; a new one for every download.
    cancel: Arc<AtomicBool>,
    shown_once: bool,
    quit: Option<Box<dyn Fn()>>,
}

/// The Download window of the app.
pub struct DownloadWindow;

impl DownloadWindow {
    /// Builds the window, hidden until the runtime waits for models or the user asks.
    pub fn install(mtm: MainThreadMarker, source: DownloadSource) -> Self {
        let window = build_window(mtm);
        let target = WindowTarget::new(mtm);
        WINDOW.set(Some(Window {
            window,
            target,
            source,
            model: None,
            cancel: Arc::new(AtomicBool::new(false)),
            shown_once: false,
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

    /// Opens the window on Waiting for models.
    pub fn observer(&self) -> DownloadWindowObserver {
        DownloadWindowObserver
    }
}

/// Brings the window to the front with a fresh check, from the menu or the settings window.
pub fn bring_to_front() {
    dispatch_to_main(|| show(true));
}

/// Tells the window each runtime state.
pub struct DownloadWindowObserver;

impl RuntimeObserver for DownloadWindowObserver {
    fn state_changed(&self, state: &RuntimeState) {
        if let RuntimeState::WaitingForModels { .. } = state {
            dispatch_to_main(|| show(false));
        }
    }
}

fn running() -> bool {
    WINDOW.with_borrow(|state| {
        state
            .as_ref()
            .and_then(|s| s.model.as_ref())
            .is_some_and(|m| m.running)
    })
}

/// Checks the files again unless a download runs, then shows the window.
fn show(to_front: bool) {
    if !running() {
        recheck();
    }
    WINDOW.with_borrow_mut(|state| {
        let Some(state) = state else { return };
        if !state.shown_once {
            state.window.center();
        }
        state.shown_once = true;
        let mtm = MainThreadMarker::from(&*state.target);
        if to_front || !state.window.isVisible() {
            NSApplication::sharedApplication(mtm).activate();
            state.window.makeKeyAndOrderFront(None);
        }
    });
}

fn recheck() {
    WINDOW.with_borrow_mut(|state| {
        let Some(state) = state else { return };
        let (download, folder) = (state.source)();
        let report = download.check();
        match &mut state.model {
            Some(model) if model.folder == folder => model.rechecked(&report),
            _ => state.model = Some(WindowModel::new(&report, &folder)),
        }
    });
    redraw();
}

fn redraw() {
    WINDOW.with_borrow(|state| {
        let Some(state) = state else { return };
        let Some(model) = &state.model else { return };
        let mtm = MainThreadMarker::from(&*state.target);
        state
            .window
            .setContentView(Some(&content(mtm, &state.target, model)));
    });
}

fn apply(event: DownloadEvent) {
    WINDOW.with_borrow_mut(|state| {
        if let Some(model) = state.as_mut().and_then(|s| s.model.as_mut()) {
            model.apply(&event);
        }
    });
    redraw();
}

fn ended() {
    WINDOW.with_borrow_mut(|state| {
        if let Some(model) = state.as_mut().and_then(|s| s.model.as_mut()) {
            model.ended();
        }
    });
    recheck();
}

/// Download, Retry or Cancel, as the button reads.
fn main_button() {
    let started = WINDOW.with_borrow_mut(|state| {
        let state = state.as_mut()?;
        let model = state.model.as_mut()?;
        if model.running {
            state.cancel.store(true, Ordering::SeqCst);
            return None;
        }
        model.started();
        state.cancel = Arc::new(AtomicBool::new(false));
        Some(((state.source)().0, Arc::clone(&state.cancel)))
    });
    redraw();
    let Some((download, cancel)) = started else {
        return;
    };
    let spawned = std::thread::Builder::new()
        .name("model-download".into())
        .spawn(move || run(download, &cancel));
    if let Err(e) = spawned {
        log::warn!("models: the download did not start: {e}");
        dispatch_to_main(ended);
    }
}

/// The download thread: every event to the main thread, progress at most every 100 ms.
fn run(download: ModelDownload, cancel: &AtomicBool) {
    let last_progress = Mutex::new(None::<Instant>);
    let summary = download.download_missing(
        &mut |event: &DownloadEvent| {
            if let DownloadEvent::Progress {
                received, total, ..
            } = event
            {
                let mut last = last_progress.lock().unwrap_or_else(|e| e.into_inner());
                let now = Instant::now();
                let due = last.is_none_or(|at| now.duration_since(at) >= PROGRESS_EVERY);
                if !due && received < total {
                    return;
                }
                *last = Some(now);
            }
            log_event(event);
            let event = event.clone();
            dispatch_to_main(move || apply(event));
        },
        cancel,
    );
    log::info!(
        "models: download ended — {} failed, cancelled {}, every file present {}",
        summary.failed.len(),
        summary.cancelled,
        summary.all_present
    );
    dispatch_to_main(ended);
}

fn log_event(event: &DownloadEvent) {
    match event {
        DownloadEvent::Progress { .. } => {}
        DownloadEvent::Failed { name, failure } => log::warn!("models: {name} failed: {failure}"),
        other => log::info!("models: {other:?}"),
    }
}

fn open_folder() {
    let folder = WINDOW.with_borrow(|state| {
        state
            .as_ref()
            .and_then(|s| s.model.as_ref())
            .map(|m| m.folder.clone())
    });
    if let Some(folder) = folder {
        if !show_in_file_browser(&folder) {
            log::warn!("models: {} not shown", folder.display());
        }
    }
}

fn quit() {
    WINDOW.with_borrow(|state| {
        if let Some(state) = state {
            state.cancel.store(true, Ordering::SeqCst);
        }
    });
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
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitle(&NSString::from_str(mapping::TITLE));
    window
}

/// Frame `height` high, `y` points below the content's top edge.
fn below_top(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(x, HEIGHT - y - height),
        NSSize::new(width, height),
    )
}

fn content(mtm: MainThreadMarker, target: &WindowTarget, model: &WindowModel) -> Retained<NSView> {
    let view = NSView::initWithFrame(
        mtm.alloc::<NSView>(),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, HEIGHT)),
    );
    let text_width = WIDTH - 2.0 * MARGIN;
    let intro = wrapping(mtm, &model.intro(), NSFont::systemFontOfSize(13.0));
    intro.setFrame(below_top(MARGIN, 14.0, text_width, 36.0));
    view.addSubview(&intro);
    for (index, row) in model.rows.iter().enumerate() {
        let top = TABLE_TOP + index as f64 * ROW_HEIGHT;
        let name = label(
            mtm,
            row.name,
            NSFont::monospacedSystemFontOfSize_weight(12.0, unsafe { NSFontWeightMedium }),
        );
        name.setFrame(below_top(MARGIN, top, SIZE_X - MARGIN, 16.0));
        view.addSubview(&name);
        let purpose = label(mtm, row.purpose, NSFont::systemFontOfSize(11.0));
        purpose.setTextColor(Some(&NSColor::secondaryLabelColor()));
        purpose.setFrame(below_top(MARGIN, top + 17.0, SIZE_X - MARGIN, 15.0));
        view.addSubview(&purpose);
        let size = label(
            mtm,
            &human_size(row.size_bytes),
            NSFont::systemFontOfSize(12.0),
        );
        size.setAlignment(NSTextAlignment::Right);
        size.setFrame(below_top(SIZE_X, top, SIZE_WIDTH, 16.0));
        view.addSubview(&size);
        let status = wrapping(
            mtm,
            &status_text(&row.status),
            NSFont::systemFontOfSize(11.0),
        );
        let colour = match row.status {
            mapping::RowStatus::Present => NSColor::systemGreenColor(),
            mapping::RowStatus::Failed(_) => NSColor::systemRedColor(),
            _ => NSColor::labelColor(),
        };
        status.setTextColor(Some(&colour));
        status.setFrame(below_top(STATUS_X, top, WIDTH - MARGIN - STATUS_X, 32.0));
        view.addSubview(&status);
    }
    let below_table = TABLE_TOP + model.rows.len() as f64 * ROW_HEIGHT + 8.0;
    let bar = NSProgressIndicator::initWithFrame(
        mtm.alloc::<NSProgressIndicator>(),
        below_top(MARGIN, below_table, text_width, 12.0),
    );
    bar.setStyle(NSProgressIndicatorStyle::Bar);
    bar.setIndeterminate(false);
    bar.setMinValue(0.0);
    bar.setMaxValue(1.0);
    bar.setDoubleValue(model.total_fraction());
    view.addSubview(&bar);
    let total = label(mtm, &model.total_line(), NSFont::systemFontOfSize(12.0));
    total.setTextColor(Some(&NSColor::secondaryLabelColor()));
    total.setFrame(below_top(MARGIN, below_table + 16.0, text_width, 16.0));
    view.addSubview(&total);
    let (space, red) = model.space_line();
    let space = label(mtm, &space, NSFont::systemFontOfSize(12.0));
    if red {
        space.setTextColor(Some(&NSColor::systemRedColor()));
    }
    space.setFrame(below_top(MARGIN, below_table + 34.0, text_width, 16.0));
    view.addSubview(&space);
    let footer = HEIGHT - 42.0;
    let open = push_button(mtm, target, mapping::OPEN_FOLDER, sel!(openFolder:));
    open.setFrame(below_top(MARGIN, footer, BUTTON_WIDTH + 10.0, 28.0));
    view.addSubview(&open);
    let quit = push_button(mtm, target, mapping::QUIT, sel!(quit:));
    quit.setFrame(below_top(
        MARGIN + BUTTON_WIDTH + 16.0,
        footer,
        BUTTON_WIDTH,
        28.0,
    ));
    view.addSubview(&quit);
    let done = push_button(mtm, target, mapping::DONE, sel!(done:));
    done.setEnabled(model.done_enabled());
    done.setFrame(below_top(
        WIDTH - MARGIN - BUTTON_WIDTH,
        footer,
        BUTTON_WIDTH,
        28.0,
    ));
    view.addSubview(&done);
    let main = model.main_button();
    let download = push_button(mtm, target, main.title(), sel!(download:));
    download.setEnabled(model.main_enabled());
    if main != MainButton::Cancel && !model.done_enabled() {
        download.setKeyEquivalent(&NSString::from_str("\r"));
    } else if model.done_enabled() {
        done.setKeyEquivalent(&NSString::from_str("\r"));
    }
    download.setFrame(below_top(
        WIDTH - MARGIN - 2.0 * BUTTON_WIDTH - 8.0,
        footer,
        BUTTON_WIDTH,
        28.0,
    ));
    view.addSubview(&download);
    view
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
) -> Retained<NSButton> {
    // SAFETY: `action` is a method of `target` taking the sender, as a button calls it.
    unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target as &AnyObject),
            Some(action),
            mtm,
        )
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPDownloadWindowTarget"]
    struct WindowTarget;

    unsafe impl NSObjectProtocol for WindowTarget {}

    impl WindowTarget {
        #[unsafe(method(download:))]
        fn download(&self, _sender: Option<&AnyObject>) {
            // not now: the button that asked is part of the content a redraw replaces
            dispatch_to_main(main_button);
        }

        #[unsafe(method(openFolder:))]
        fn open_folder(&self, _sender: Option<&AnyObject>) {
            open_folder();
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            quit();
        }

        #[unsafe(method(done:))]
        fn done(&self, _sender: Option<&AnyObject>) {
            close();
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
