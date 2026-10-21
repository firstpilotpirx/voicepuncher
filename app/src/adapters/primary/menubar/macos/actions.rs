//! What the menu's items do: quit the app, open the settings window, show the logs folder,
//! bring the Permissions window to the front.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use objc2::MainThreadMarker;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSString;

use crate::adapters::primary::settings::macos::window::SettingsWindow;
use crate::runtime::Runtime;

/// The process's exit code after Quit.
pub const QUIT_EXIT_CODE: i32 = 0;

/// The menu's actions; called on the main thread when the user chooses an item.
pub trait MenuActions {
    fn quit(&self);
    fn open_settings(&self);
    fn open_logs(&self);
    fn open_permissions(&self);
}

/// The same actions, shared by the menu and the Permissions window.
impl<T: MenuActions + ?Sized> MenuActions for std::rc::Rc<T> {
    fn quit(&self) {
        (**self).quit();
    }

    fn open_settings(&self) {
        (**self).open_settings();
    }

    fn open_logs(&self) {
        (**self).open_logs();
    }

    fn open_permissions(&self) {
        (**self).open_permissions();
    }
}

/// Opens the settings window; called on the main thread.
pub trait SettingsOpener {
    fn open(&self);
}

impl SettingsOpener for SettingsWindow {
    fn open(&self) {
        let Some(mtm) = MainThreadMarker::new() else {
            log::warn!("menu: settings asked for off the main thread");
            return;
        };
        if let Err(e) = self.show(mtm) {
            log::warn!("menu: settings not opened: {e:#}");
        }
    }
}

/// The actions of the app: Quit owns the runtime it stops.
pub struct MacosMenuActions {
    runtime: RefCell<Option<Runtime>>,
    settings: Box<dyn SettingsOpener>,
    logs_dir: PathBuf,
    end_process: Box<dyn Fn(i32)>,
    open_permissions: Box<dyn Fn()>,
}

impl MacosMenuActions {
    /// Quit ends the process.
    pub fn new(runtime: Runtime, settings: Box<dyn SettingsOpener>, logs_dir: PathBuf) -> Self {
        Self::ending_with(
            runtime,
            settings,
            logs_dir,
            Box::new(|code| std::process::exit(code)),
        )
    }

    /// Quit calls `end_process` with the exit code once the runtime stopped.
    pub fn ending_with(
        runtime: Runtime,
        settings: Box<dyn SettingsOpener>,
        logs_dir: PathBuf,
        end_process: Box<dyn Fn(i32)>,
    ) -> Self {
        Self {
            runtime: RefCell::new(Some(runtime)),
            settings,
            logs_dir,
            end_process,
            open_permissions: Box::new(|| log::warn!("menu: no Permissions window")),
        }
    }

    /// Permissions… calls `open`.
    pub fn with_permissions_window(mut self, open: Box<dyn Fn()>) -> Self {
        self.open_permissions = open;
        self
    }
}

impl MenuActions for MacosMenuActions {
    /// Stops the runtime, cancelling a recording in progress, then ends the process.
    fn quit(&self) {
        if let Some(mut runtime) = self.runtime.borrow_mut().take() {
            runtime.stop();
        }
        (self.end_process)(QUIT_EXIT_CODE);
    }

    fn open_settings(&self) {
        self.settings.open();
    }

    fn open_permissions(&self) {
        (self.open_permissions)();
    }

    fn open_logs(&self) {
        if !show_in_file_browser(&self.logs_dir) {
            log::warn!("menu: logs folder {} not shown", self.logs_dir.display());
        }
    }
}

/// Creates `folder` when it does not exist yet.
pub fn ensure_folder(folder: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(folder)
}

/// Shows `folder` in the file browser, creating it first; false when it could not be shown.
pub fn show_in_file_browser(folder: &Path) -> bool {
    if let Err(e) = ensure_folder(folder) {
        log::warn!("menu: cannot create {}: {e}", folder.display());
        return false;
    }
    let path = NSString::from_str(&folder.to_string_lossy());
    NSWorkspace::sharedWorkspace().selectFile_inFileViewerRootedAtPath(Some(&path), &path)
}
