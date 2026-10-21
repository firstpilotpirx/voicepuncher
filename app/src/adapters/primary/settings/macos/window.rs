//! The settings window as the menu opens it: one at a time, brought to the front when it is
//! open already, built afresh from the file when it was closed.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use objc2::runtime::ProtocolObject;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSWindow, NSWindowToolbarStyle};
use objc2_foundation::NSString;

use super::build::{control, Parts};
use super::form::Form;
use super::layout::tabs;
use super::session::SettingsSession;
use super::table::FIELDS;
use super::target::FormTarget;

const TITLE: &str = "voicepuncher Settings";

/// Opens the settings session of the configuration file each time the window opens.
pub type SessionOpener = Box<dyn Fn() -> Result<SettingsSession>>;

/// The settings window; it lives on the main thread.
pub struct SettingsWindow {
    opener: SessionOpener,
    logs_dir: PathBuf,
    shown: RefCell<Option<Rc<Form>>>,
}

impl SettingsWindow {
    pub fn new(opener: SessionOpener, logs_dir: PathBuf) -> Self {
        Self {
            opener,
            logs_dir,
            shown: RefCell::new(None),
        }
    }

    /// Brings the window to the front, building it first when it is not open.
    pub fn show(&self, mtm: MainThreadMarker) -> Result<()> {
        let form = self.prepare(mtm)?;
        form.window.makeKeyAndOrderFront(None);
        NSApplication::sharedApplication(mtm).activate();
        Ok(())
    }

    /// The open window, or a new one built from the file when none is open; not shown.
    pub fn prepare(&self, mtm: MainThreadMarker) -> Result<Rc<Form>> {
        if let Some(form) = self.form().filter(|form| form.window.isVisible()) {
            return Ok(form);
        }
        let form = build(mtm, (self.opener)()?, self.logs_dir.clone());
        self.shown.replace(Some(Rc::clone(&form)));
        Ok(form)
    }

    /// The window built last, open or closed.
    pub fn form(&self) -> Option<Rc<Form>> {
        self.shown.borrow().clone()
    }

    /// The window's own window, when it was built.
    pub fn window(&self) -> Option<objc2::rc::Retained<NSWindow>> {
        self.form().map(|form| form.window.clone())
    }
}

fn build(mtm: MainThreadMarker, session: SettingsSession, logs_dir: PathBuf) -> Rc<Form> {
    let target = FormTarget::new(mtm);
    let parts = Parts {
        mtm,
        target: &target,
        session: &session,
    };
    let controls: Vec<_> = FIELDS
        .iter()
        .enumerate()
        .map(|(tag, field)| control(&parts, field, tag as isize))
        .collect();
    let notes: Vec<(usize, String)> = session
        .notes()
        .into_iter()
        .filter_map(|note| {
            let place = FIELDS.iter().position(|f| f.path() == note.field.path())?;
            Some((place, note.note))
        })
        .collect();
    let (controller, tab_views) = tabs(&parts, &controls, &notes);
    let window = NSWindow::windowWithContentViewController(&controller);
    window.setTitle(&NSString::from_str(TITLE));
    window.setToolbarStyle(NSWindowToolbarStyle::Preference);
    // SAFETY: the form keeps the window; AppKit must not release it on close.
    unsafe { window.setReleasedWhenClosed(false) };
    window.setDelegate(Some(ProtocolObject::from_ref(&*target)));
    window.center();
    let form = Rc::new(Form {
        window,
        controller,
        target,
        session: RefCell::new(session),
        controls,
        tabs: tab_views,
        logs_dir,
    });
    Form::connect(&form);
    form
}
