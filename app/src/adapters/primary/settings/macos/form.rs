//! The open settings window: its controls over one settings session, and what Save,
//! Revert, closing and the buttons beside the controls do.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSColor, NSModalResponseOK,
    NSOpenPanel, NSSound, NSTabViewController, NSWindow,
};
use objc2_foundation::NSString;
use toml::Value;

use super::control::{two_decimals, FieldControl, Widget};
use super::files::{collapse_home, reveal};
use super::layout::TabViews;
use super::login::refusal_text;
use super::login_row::{open_login_items, LoginRow};
use super::session::{status_text, ApplyStatus, SettingsSession};
use super::target::{Event, EventHandler, FormTarget};
use crate::adapters::main_queue::dispatch_to_main;

const UNSAVED_MESSAGE: &str = "You have unsaved changes.";
const SAVE_TITLE: &str = "Save";
const DONT_SAVE_TITLE: &str = "Don't Save";
const CANCEL_TITLE: &str = "Cancel";
const SAVED: &str = "Saved";

thread_local! {
    /// The open window's form, reached by the status reports the main queue delivers.
    static SHOWN: RefCell<Weak<Form>> = const { RefCell::new(Weak::new()) };
}

/// The window, its controls and the session they edit.
pub struct Form {
    pub window: Retained<NSWindow>,
    pub controller: Retained<NSTabViewController>,
    pub target: Retained<FormTarget>,
    pub session: RefCell<SettingsSession>,
    pub controls: Vec<FieldControl>,
    pub login: LoginRow,
    pub tabs: Vec<TabViews>,
    pub logs_dir: PathBuf,
}

impl Form {
    /// Makes `form` hear its controls and the status reports of its saves.
    pub fn connect(form: &Rc<Form>) {
        form.target.connect(Box::new(Handler(Rc::downgrade(form))));
        SHOWN.with(|shown| shown.replace(Rc::downgrade(form)));
    }

    /// Reads every control into the session.
    fn collect(&self) {
        let mut session = self.session.borrow_mut();
        for control in &self.controls {
            session.edit(control.field, control.read());
        }
    }

    /// Saves; false when a value was refused and is marked.
    fn save(&self) -> bool {
        self.collect();
        self.controls.iter().for_each(|control| control.mark(false));
        let listener = Box::new(|status: ApplyStatus| {
            dispatch_to_main(move || {
                if let Some(form) = SHOWN.with(|shown| shown.borrow().upgrade()) {
                    form.say(
                        &status_text(&status),
                        matches!(status, ApplyStatus::Rejected(_)),
                    );
                }
            })
        });
        self.say(SAVED, false);
        let saved = self.session.borrow_mut().save(listener);
        match saved {
            Ok(()) => true,
            Err(refusal) => {
                if let Some(control) = refusal.field.and_then(|field| {
                    self.controls
                        .iter()
                        .find(|c| c.field.path() == field.path())
                }) {
                    control.mark(true);
                }
                self.say(&refusal.message, true);
                false
            }
        }
    }

    fn revert(&self) {
        self.session.borrow_mut().revert();
        let session = self.session.borrow();
        for control in &self.controls {
            control.show(&session.value(control.field));
            control.mark(false);
        }
        self.say("", false);
    }

    /// Writes `text` on every tab's status line, in red for an error.
    fn say(&self, text: &str, error: bool) {
        let color = if error {
            NSColor::systemRedColor()
        } else {
            NSColor::secondaryLabelColor()
        };
        for tab in &self.tabs {
            tab.status.setStringValue(&NSString::from_str(text));
            tab.status.setTextColor(Some(&color));
        }
    }

    /// Whether the window may close: asks first when an edit is unsaved.
    pub fn should_close(&self) -> bool {
        self.collect();
        if !self.session.borrow().is_dirty() {
            return true;
        }
        let mtm = self.target.mtm();
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(UNSAVED_MESSAGE));
        for title in [SAVE_TITLE, DONT_SAVE_TITLE, CANCEL_TITLE] {
            alert.addButtonWithTitle(&NSString::from_str(title));
        }
        match alert.runModal() {
            answer if answer == NSAlertFirstButtonReturn => self.save(),
            answer if answer == NSAlertSecondButtonReturn => {
                self.revert();
                true
            }
            _ => false,
        }
    }

    /// Does what the user's `event` asks.
    pub fn handle(&self, event: Event) {
        match event {
            Event::Changed(tag) => self.changed(tag, false),
            Event::Stepped(tag) => self.changed(tag, true),
            Event::Play(tag) => self.play(tag),
            Event::Choose(tag) => self.choose(tag),
            Event::Reveal(tag) => self.with_control(tag, |control| {
                reveal(&super::files::resolve(&string(&control.read())));
            }),
            Event::AddExample => self.with_examples(|table| table.add()),
            Event::RemoveExample => self.with_examples(|table| table.remove_selected()),
            Event::ToggleAdvanced(place) => self.toggle_advanced(place),
            Event::Save => {
                self.save();
            }
            Event::Revert => self.revert(),
            Event::OpenLogs => reveal(&self.logs_dir),
            Event::LoginToggled => self.login_toggled(),
            Event::OpenLoginItems => open_login_items(),
            Event::BecameKey => self.login.refresh(),
        }
    }

    fn login_toggled(&self) {
        match self.login.turned() {
            Some(reason) => self.say(&refusal_text(&reason), true),
            None => self.say("", false),
        }
    }

    /// Keeps a number's text and stepper, and a slider's label, in step.
    fn changed(&self, tag: isize, stepped: bool) {
        self.with_control(tag, |control| match &control.widget {
            Widget::Number { text, stepper } if stepped => {
                text.setStringValue(&NSString::from_str(&stepper.integerValue().to_string()))
            }
            Widget::Number { text, stepper } => stepper.setDoubleValue(text.doubleValue()),
            Widget::Slider { slider, shown } => {
                shown.setStringValue(&NSString::from_str(&two_decimals(slider.doubleValue())))
            }
            _ => {}
        });
        self.collect();
    }

    fn play(&self, tag: isize) {
        self.with_control(tag, |control| {
            let name = string(&control.read());
            if let Some(sound) = NSSound::soundNamed(&NSString::from_str(&name)) {
                sound.play();
            }
        });
    }

    fn choose(&self, tag: isize) {
        let panel = NSOpenPanel::openPanel(self.target.mtm());
        panel.setCanChooseDirectories(true);
        panel.setCanChooseFiles(false);
        panel.setCanCreateDirectories(true);
        if panel.runModal() != NSModalResponseOK {
            return;
        }
        let Some(path) = panel.URL().and_then(|url| url.path()) else {
            return;
        };
        let folder = collapse_home(&PathBuf::from(path.to_string()));
        self.with_control(tag, |control| control.show(&Value::String(folder)));
        self.collect();
    }

    fn toggle_advanced(&self, place: isize) {
        let Some(tab) = usize::try_from(place).ok().and_then(|p| self.tabs.get(p)) else {
            return;
        };
        let hidden = tab.advanced.first().is_some_and(|row| row.isHidden());
        tab.advanced.iter().for_each(|row| row.setHidden(!hidden));
        tab.fit();
    }

    fn with_control(&self, tag: isize, act: impl FnOnce(&FieldControl)) {
        if let Some(control) = usize::try_from(tag).ok().and_then(|t| self.controls.get(t)) {
            act(control);
        }
    }

    fn with_examples(&self, act: impl FnOnce(&super::examples::ExamplesTable)) {
        let table = self
            .controls
            .iter()
            .find_map(|control| match &control.widget {
                Widget::Examples(table) => Some(table),
                _ => None,
            });
        if let Some(table) = table {
            act(table);
        }
    }

    /// Shows the tab at `place` in the toolbar.
    pub fn select_tab(&self, place: usize) {
        self.controller.setSelectedTabViewItemIndex(place as isize);
    }

    /// The status line of the first tab, as the window shows it.
    pub fn status_line(&self) -> String {
        self.tabs
            .first()
            .map(|tab| tab.status.stringValue().to_string())
            .unwrap_or_default()
    }
}

fn string(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

/// Hands the target's events to the form while it is open.
struct Handler(Weak<Form>);

impl EventHandler for Handler {
    fn handle(&self, event: Event) {
        if let Some(form) = self.0.upgrade() {
            form.handle(event);
        }
    }

    fn should_close(&self) -> bool {
        self.0.upgrade().is_none_or(|form| form.should_close())
    }
}
