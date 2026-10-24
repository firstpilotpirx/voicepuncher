//! The login row of the General tab: the switch, its note and the button to the Login
//! Items settings, kept to what the system says about the login item.

use objc2::rc::Retained;
use objc2_app_kit::{
    NSButton, NSColor, NSControlStateValueOff, NSControlStateValueOn, NSFont, NSSwitch,
    NSTextField, NSView, NSWorkspace,
};
use objc2_foundation::{NSString, NSURL};

use super::build::{button, stack, Parts};
use super::login::{
    LoginSwitch, LoginSwitchView, Turned, LOGIN_ITEMS_SETTINGS, OPEN_LOGIN_ITEMS_TITLE,
};
use super::target::Action;
use crate::adapters::primary::settings::hints::switch_alpha;

const NOTE_SIZE: f64 = 11.0;

/// The row's views over the login switch.
pub struct LoginRow {
    pub switch: LoginSwitch,
    pub control: Retained<NSSwitch>,
    pub note: Retained<NSTextField>,
    pub open_settings: Retained<NSButton>,
    /// What the grid holds: the switch with the button beside it.
    pub view: Retained<NSView>,
}

impl LoginRow {
    pub fn new(parts: &Parts, switch: LoginSwitch) -> Self {
        let control = NSSwitch::new(parts.mtm);
        parts.target.wire(&control, Action::LOGIN_TOGGLED, 0);
        let open_settings = button(parts, OPEN_LOGIN_ITEMS_TITLE, Action::OPEN_LOGIN_ITEMS, 0);
        let note = NSTextField::labelWithString(&NSString::new(), parts.mtm);
        note.setFont(Some(&NSFont::systemFontOfSize(NOTE_SIZE)));
        note.setTextColor(Some(&NSColor::secondaryLabelColor()));
        let view = stack(parts.mtm, &[control.as_ref(), open_settings.as_ref()]);
        let row = Self { switch, control, note, open_settings, view: Retained::into_super(view) };
        row.refresh();
        row
    }

    /// Shows what the system says now.
    pub fn refresh(&self) {
        self.show(&self.switch.view());
    }

    /// The user turned the switch; the reason when the system refused.
    pub fn turned(&self) -> Option<String> {
        let Turned { view, refusal } = self.switch.turn(self.is_on());
        self.show(&view);
        refusal
    }

    /// Whether the switch is on, as the window shows it.
    pub fn is_on(&self) -> bool {
        self.control.state() == NSControlStateValueOn
    }

    fn show(&self, view: &LoginSwitchView) {
        let states = [NSControlStateValueOff, NSControlStateValueOn];
        self.control.setState(states[usize::from(view.on)]);
        self.control.setEnabled(view.enabled);
        self.control.setAlphaValue(switch_alpha(view.enabled));
        self.note
            .setStringValue(&NSString::from_str(view.note.unwrap_or_default()));
        self.note.setHidden(view.note.is_none());
        self.open_settings.setHidden(!view.approval_button);
    }
}

/// Opens System Settings → General → Login Items.
pub fn open_login_items() {
    let Some(url) = NSURL::URLWithString(&NSString::from_str(LOGIN_ITEMS_SETTINGS)) else {
        return;
    };
    if !NSWorkspace::sharedWorkspace().openURL(&url) {
        log::warn!("settings: the Login Items settings did not open");
    }
}
