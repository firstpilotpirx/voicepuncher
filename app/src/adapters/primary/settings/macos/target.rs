//! The object `AppKit` calls when the user works a control or closes the window; it hands each
//! call on as an [`Event`].

use std::cell::OnceCell;
use std::ffi::CStr;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSControl, NSWindow, NSWindowDelegate};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol};

/// What the user did; the number is the tag of the control: the field's place in the
/// field table, or the tab's place in the toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Changed(isize),
    Stepped(isize),
    Play(isize),
    Choose(isize),
    Reveal(isize),
    ToggleAdvanced(isize),
    Save,
    Revert,
    OpenLogs,
    LoginToggled,
    OpenLoginItems,
    DownloadModels,
    ShowModels,
    /// The window came to the front: what the system keeps may have changed meanwhile.
    BecameKey,
}

/// Handles the events and answers whether the window may close.
pub trait EventHandler {
    fn handle(&self, event: Event);
    fn should_close(&self) -> bool;
}

pub struct FormTargetIvars {
    handler: OnceCell<Box<dyn EventHandler>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPSettingsFormTarget"]
    #[ivars = FormTargetIvars]
    pub struct FormTarget;

    unsafe impl NSObjectProtocol for FormTarget {}

    unsafe impl NSWindowDelegate for FormTarget {
        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _window: &NSWindow) -> bool {
            self.ivars()
                .handler
                .get()
                .is_none_or(|handler| handler.should_close())
        }

        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _notification: &NSNotification) {
            self.send(Event::BecameKey);
        }
    }

    impl FormTarget {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: Option<&AnyObject>) {
            self.send_tagged(Event::Changed, sender);
        }

        #[unsafe(method(stepped:))]
        fn stepped(&self, sender: Option<&AnyObject>) {
            self.send_tagged(Event::Stepped, sender);
        }

        #[unsafe(method(play:))]
        fn play(&self, sender: Option<&AnyObject>) {
            self.send_tagged(Event::Play, sender);
        }

        #[unsafe(method(choose:))]
        fn choose(&self, sender: Option<&AnyObject>) {
            self.send_tagged(Event::Choose, sender);
        }

        #[unsafe(method(reveal:))]
        fn reveal(&self, sender: Option<&AnyObject>) {
            self.send_tagged(Event::Reveal, sender);
        }

        #[unsafe(method(toggleAdvanced:))]
        fn toggle_advanced(&self, sender: Option<&AnyObject>) {
            self.send_tagged(Event::ToggleAdvanced, sender);
        }

        #[unsafe(method(save:))]
        fn save(&self, _sender: Option<&AnyObject>) {
            self.send(Event::Save);
        }

        #[unsafe(method(revert:))]
        fn revert(&self, _sender: Option<&AnyObject>) {
            self.send(Event::Revert);
        }

        #[unsafe(method(openLogs:))]
        fn open_logs(&self, _sender: Option<&AnyObject>) {
            self.send(Event::OpenLogs);
        }

        #[unsafe(method(loginToggled:))]
        fn login_toggled(&self, _sender: Option<&AnyObject>) {
            self.send(Event::LoginToggled);
        }

        #[unsafe(method(openLoginItems:))]
        fn open_login_items(&self, _sender: Option<&AnyObject>) {
            self.send(Event::OpenLoginItems);
        }

        #[unsafe(method(downloadModels:))]
        fn download_models(&self, _sender: Option<&AnyObject>) {
            self.send(Event::DownloadModels);
        }

        #[unsafe(method(showModels:))]
        fn show_models(&self, _sender: Option<&AnyObject>) {
            self.send(Event::ShowModels);
        }
    }
);

impl FormTarget {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FormTargetIvars { handler: OnceCell::new() });
        // SAFETY: `NSObject`'s `init` on a freshly allocated object.
        unsafe { msg_send![super(this), init] }
    }

    /// Who hears the events; set once, after the controls exist.
    pub fn connect(&self, handler: Box<dyn EventHandler>) {
        let _ = self.ivars().handler.set(handler);
    }

    /// Sends the event of a control's tag; nothing for a sender that is no control.
    fn send_tagged(&self, event: fn(isize) -> Event, sender: Option<&AnyObject>) {
        if let Some(tag) = tag(sender) {
            self.send(event(tag));
        }
    }

    fn send(&self, event: Event) {
        if let Some(handler) = self.ivars().handler.get() {
            handler.handle(event);
        }
    }

    /// Makes `control` call `action` of this target, tagged with `tag`.
    pub fn wire(&self, control: &NSControl, action: Action, tag: isize) {
        // SAFETY: the target lives as long as the window that holds the control.
        unsafe { control.setTarget(Some(self as &AnyObject)) };
        // SAFETY: `action` is a method of this target taking the sender.
        unsafe { control.setAction(Some(action.selector())) };
        control.setTag(tag);
    }
}

/// What a control asks the target to do: the name of one of the target's methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action(&'static CStr);

impl Action {
    pub const CHANGED: Self = Self(c"changed:");
    pub const STEPPED: Self = Self(c"stepped:");
    pub const PLAY: Self = Self(c"play:");
    pub const CHOOSE: Self = Self(c"choose:");
    pub const REVEAL: Self = Self(c"reveal:");
    pub const TOGGLE_ADVANCED: Self = Self(c"toggleAdvanced:");
    pub const SAVE: Self = Self(c"save:");
    pub const REVERT: Self = Self(c"revert:");
    pub const OPEN_LOGS: Self = Self(c"openLogs:");
    pub const LOGIN_TOGGLED: Self = Self(c"loginToggled:");
    pub const OPEN_LOGIN_ITEMS: Self = Self(c"openLoginItems:");
    pub const DOWNLOAD_MODELS: Self = Self(c"downloadModels:");
    pub const SHOW_MODELS: Self = Self(c"showModels:");

    /// The selector of the target's method for the action.
    pub fn selector(self) -> Sel {
        Sel::register(self.0)
    }
}

/// The tag of the control that sent an action; none for a sender that is no control.
fn tag(sender: Option<&AnyObject>) -> Option<isize> {
    sender
        .and_then(|sender| sender.downcast_ref::<NSControl>())
        .map(NSControl::tag)
}

#[cfg(test)]
mod tests {
    use objc2::runtime::NSObject;

    use super::*;

    #[test]
    fn should_give_no_tag_for_a_sender_that_is_no_control() {
        let sender = NSObject::new();
        assert_eq!((tag(Some(&sender)), tag(None)), (None, None));
    }
}
