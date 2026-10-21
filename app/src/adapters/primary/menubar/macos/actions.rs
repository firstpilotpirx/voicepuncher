//! What the menu's items do: quit the app, open the settings, show the logs folder.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL};

use crate::runtime::Runtime;

/// The process's exit code after Quit.
pub const QUIT_EXIT_CODE: i32 = 0;
/// Opens a file with the system's plain text editor: `open -t`.
const OPEN_COMMAND: &str = "open";
const PLAIN_TEXT_EDITOR_FLAG: &str = "-t";

/// The menu's actions; called on the main thread when the user chooses an item.
pub trait MenuActions {
    fn quit(&self);
    fn open_settings(&self);
    fn open_logs(&self);
}

/// The actions of the app: Quit owns the runtime it stops.
pub struct MacosMenuActions {
    runtime: RefCell<Option<Runtime>>,
    config_path: PathBuf,
    logs_dir: PathBuf,
    end_process: Box<dyn Fn(i32)>,
}

impl MacosMenuActions {
    /// Quit ends the process.
    pub fn new(runtime: Runtime, config_path: PathBuf, logs_dir: PathBuf) -> Self {
        Self::ending_with(
            runtime,
            config_path,
            logs_dir,
            Box::new(|code| std::process::exit(code)),
        )
    }

    /// Quit calls `end_process` with the exit code once the runtime stopped.
    pub fn ending_with(
        runtime: Runtime,
        config_path: PathBuf,
        logs_dir: PathBuf,
        end_process: Box<dyn Fn(i32)>,
    ) -> Self {
        Self {
            runtime: RefCell::new(Some(runtime)),
            config_path,
            logs_dir,
            end_process,
        }
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
        if let Err(e) = open_in_text_editor(&self.config_path) {
            log::warn!("menu: settings not opened: {e:#}");
        }
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
