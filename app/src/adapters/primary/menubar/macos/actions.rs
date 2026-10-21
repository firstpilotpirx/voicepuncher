//! What the menu's items do: quit the app, open the settings window, show the logs folder,
//! bring the Permissions window to the front.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use objc2::MainThreadMarker;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL};

use crate::adapters::primary::settings::macos::window::SettingsWindow;
use crate::core::domain::ServiceStatus;
use crate::runtime::{Runtime, ServicesView};

/// The process's exit code after Quit.
pub const QUIT_EXIT_CODE: i32 = 0;

/// The menu's actions; called on the main thread when the user chooses an item.
pub trait MenuActions {
    fn quit(&self);
    fn open_settings(&self);
    fn open_logs(&self);
    fn open_permissions(&self);

    /// The local services and their states; none when the app leaves them to the scripts.
    fn services(&self) -> Option<Vec<ServiceStatus>> {
        None
    }

    /// Starts the service on `port` again, or retries it after it failed.
    fn restart_service(&self, _port: u16) {}

    /// Shows the log file of the service on `port`.
    fn open_service_log(&self, _port: u16) {}
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

    fn services(&self) -> Option<Vec<ServiceStatus>> {
        (**self).services()
    }

    fn restart_service(&self, port: u16) {
        (**self).restart_service(port);
    }

    fn open_service_log(&self, port: u16) {
        (**self).open_service_log(port);
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
    services: Option<std::sync::Arc<dyn ServicesView>>,
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
            services: None,
        }
    }

    /// The Services submenu lists and restarts `services`.
    pub fn with_services(mut self, services: std::sync::Arc<dyn ServicesView>) -> Self {
        self.services = Some(services);
        self
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

    fn services(&self) -> Option<Vec<ServiceStatus>> {
        self.services.as_ref().map(|services| services.statuses())
    }

    fn restart_service(&self, port: u16) {
        if let Some(services) = &self.services {
            log::info!("menu: restart of the service on port {port}");
            services.restart(port);
        }
    }

    fn open_service_log(&self, port: u16) {
        let log_file = self
            .services()
            .unwrap_or_default()
            .into_iter()
            .find(|status| status.port == port)
            .map(|status| status.log_file);
        let Some(log_file) = log_file else {
            return;
        };
        if let Err(e) = open_in_text_editor(&log_file) {
            log::warn!("menu: service log not opened: {e:#}");
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

const OPEN_COMMAND: &str = "open";
const PLAIN_TEXT_EDITOR_FLAG: &str = "-t";

/// Opens `file` in the editor the system uses for it, or the plain text editor when none is set.
pub fn open_in_text_editor(file: &Path) -> Result<()> {
    let path = NSString::from_str(&file.to_string_lossy());
    let url = NSURL::fileURLWithPath(&path);
    if NSWorkspace::sharedWorkspace().openURL(&url) {
        return Ok(());
    }
    let status = Command::new(OPEN_COMMAND)
        .arg(PLAIN_TEXT_EDITOR_FLAG)
        .arg(file)
        .status()
        .with_context(|| format!("cannot run {OPEN_COMMAND}"))?;
    if !status.success() {
        bail!(
            "{} not opened in the plain text editor: {status}",
            file.display()
        );
    }
    Ok(())
}
