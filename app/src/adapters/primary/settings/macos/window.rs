//! The settings window as the menu opens it: one at a time, brought to the front when it is
//! open already, built afresh from the file when it was closed.

use crate::core::domain::conversions::index_isize;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use objc2::runtime::ProtocolObject;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSWindow, NSWindowToolbarStyle};
use objc2_foundation::NSString;

use super::build::{control, Parts};
use super::form::Form;
use super::layout::{tabs, TabContent};
use super::login::LoginSwitch;
use super::login_row::LoginRow;
use super::models_row::{ModelsPanel, ModelsRow};
use super::session::SettingsSession;
use super::table::FIELDS;
use super::target::FormTarget;
use crate::adapters::primary::settings::hints::placed_notes;
use crate::core::application::LoginItem;
use crate::core::domain::Permission;

const TITLE: &str = "voicepuncher Settings";

/// The permissions the runtime waits for at this moment.
pub type PermissionsWaitedFor = Box<dyn Fn() -> Vec<Permission>>;

/// Opens the settings session of the configuration file each time the window opens.
pub type SessionOpener = Box<dyn Fn() -> Result<SettingsSession>>;

/// The settings window; it lives on the main thread.
pub struct SettingsWindow {
    opener: SessionOpener,
    logs_dir: PathBuf,
    login_item: Arc<dyn LoginItem>,
    models: Option<Rc<dyn ModelsPanel>>,
    permissions_wait: PermissionsWaitedFor,
    shown: RefCell<Option<Rc<Form>>>,
}

impl SettingsWindow {
    pub fn new(opener: SessionOpener, logs_dir: PathBuf, login_item: Arc<dyn LoginItem>) -> Self {
        let permissions_wait: PermissionsWaitedFor = Box::new(Vec::new);
        let (models, shown) = (None, RefCell::new(None));
        Self { opener, logs_dir, login_item, models, permissions_wait, shown }
    }

    /// The voice control switch notes the permissions `wait` holds when the window opens.
    pub fn with_permissions_wait(mut self, wait: PermissionsWaitedFor) -> Self {
        self.permissions_wait = wait;
        self
    }

    /// The Services tab shows the models of `panel`; without one it shows no Models row.
    pub fn with_models(mut self, panel: Rc<dyn ModelsPanel>) -> Self {
        self.models = Some(panel);
        self
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
        let form = self.build(mtm, (self.opener)()?);
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

impl SettingsWindow {
    fn build(&self, mtm: MainThreadMarker, session: SettingsSession) -> Rc<Form> {
        let target = FormTarget::new(mtm);
        let parts = Parts { mtm, target: &target, session: &session };
        let controls: Vec<_> = FIELDS
            .iter()
            .enumerate()
            .map(|(tag, field)| control(&parts, field, index_isize(tag)))
            .collect();
        let notes = placed_notes(session.notes(), &(self.permissions_wait)());
        let login = LoginRow::new(&parts, LoginSwitch::new(Arc::clone(&self.login_item)));
        let models = ModelsRow::new(&parts, self.models.clone());
        let (controller, tab_views) = tabs(
            &parts,
            TabContent { controls: &controls, notes: &notes, login: &login, models: &models },
        );
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
            login,
            models,
            tabs: tab_views,
            logs_dir: self.logs_dir.clone(),
        });
        Form::connect(&form);
        form
    }
}
