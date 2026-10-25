//! The Download window.
//!
//! One row per model file the configuration needs with its purpose, size and status, one bar for
//! the total, the free space, and Open Folder, Quit, Download (Cancel while it runs, Retry after a
//! failure) and Done.
//!
//! It opens when the runtime waits for models, and from the menu and the Services tab at any time;
//! the download runs on a thread of its own and every event is drawn on the main thread.

pub mod mapping;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSButton, NSColor, NSFont, NSFontWeightMedium, NSProgressIndicator,
    NSProgressIndicatorStyle, NSTextAlignment, NSView, NSWindow,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSRect, NSString};

use crate::adapters::appkit::{self, label, push_button, wrapping};
use crate::adapters::main_queue::dispatch_to_main;
use crate::adapters::primary::menubar::macos::actions::show_in_file_browser;
use crate::core::application::{DownloadEvent, ModelDownload};
use crate::runtime::{RuntimeObserver, RuntimeState};

use mapping::{human_size, status_text, MainButton, WindowModel};

mod view;
use view::content;

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

appkit::kept_window_slot!(ShownWindow);

struct ShownWindow {
    window: Retained<NSWindow>,
    target: Retained<WindowTarget>,
    source: DownloadSource,
    model: Option<WindowModel>,
    /// Set by Cancel; a new one for every download.
    cancel: Arc<AtomicBool>,
    shown_once: bool,
    quit: Option<appkit::QuitAction>,
}

/// The Download window of the app.
pub struct DownloadWindow;

impl DownloadWindow {
    /// Builds the window, hidden until the runtime waits for models or the user asks.
    pub fn install(mtm: MainThreadMarker, source: DownloadSource) -> Self {
        let window = build_window(mtm);
        let target = WindowTarget::new(mtm);
        WINDOW.set(Some(ShownWindow {
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
    pub fn set_quit(&self, quit: appkit::QuitAction) {
        appkit::set_quit(&WINDOW, quit);
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

fn apply(event: &DownloadEvent) {
    WINDOW.with_borrow_mut(|state| {
        if let Some(model) = state.as_mut().and_then(|s| s.model.as_mut()) {
            model.apply(event);
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
        .spawn(move || run(&download, &cancel));
    if let Err(e) = spawned {
        log::warn!("models: the download did not start: {e}");
        dispatch_to_main(ended);
    }
}

/// The download thread: every event to the main thread, progress at most every 100 ms.
fn run(download: &ModelDownload, cancel: &AtomicBool) {
    let last_progress = Mutex::new(None::<Instant>);
    let summary = download.download_missing(
        &mut |event: &DownloadEvent| {
            if let DownloadEvent::Progress { received, total, .. } = event {
                let mut last = last_progress
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let now = Instant::now();
                let due = last.is_none_or(|at| now.duration_since(at) >= PROGRESS_EVERY);
                if !due && received < total {
                    return;
                }
                *last = Some(now);
            }
            log_event(event);
            let event = event.clone();
            dispatch_to_main(move || apply(&event));
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
    appkit::quit(&WINDOW);
}

fn close() {
    appkit::close(&WINDOW);
}

fn build_window(mtm: MainThreadMarker) -> Retained<NSWindow> {
    appkit::titled_window(mtm, mapping::TITLE, WIDTH, HEIGHT)
}

/// Frame `height` high, `y` points below the content's top edge.
fn below_top(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    appkit::below_top(HEIGHT, x, y, width, height)
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
        appkit::new_plain_object(mtm)
    }
}
