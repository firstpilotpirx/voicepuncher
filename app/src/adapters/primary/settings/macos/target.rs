//! The object `AppKit` calls when the user works a control or closes the window; it hands each
//! call on as an [`Event`].

use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
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
    AddExample,
    RemoveExample,
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
            self.send(Event::Changed(tag(sender)));
        }

        #[unsafe(method(stepped:))]
        fn stepped(&self, sender: Option<&AnyObject>) {
            self.send(Event::Stepped(tag(sender)));
        }

        #[unsafe(method(play:))]
        fn play(&self, sender: Option<&AnyObject>) {
            self.send(Event::Play(tag(sender)));
        }

        #[unsafe(method(choose:))]
        fn choose(&self, sender: Option<&AnyObject>) {
            self.send(Event::Choose(tag(sender)));
        }

        #[unsafe(method(reveal:))]
        fn reveal(&self, sender: Option<&AnyObject>) {
            self.send(Event::Reveal(tag(sender)));
        }

        #[unsafe(method(addExample:))]
        fn add_example(&self, _sender: Option<&AnyObject>) {
            self.send(Event::AddExample);
        }

        #[unsafe(method(removeExample:))]
        fn remove_example(&self, _sender: Option<&AnyObject>) {
            self.send(Event::RemoveExample);
        }

        #[unsafe(method(toggleAdvanced:))]
        fn toggle_advanced(&self, sender: Option<&AnyObject>) {
            self.send(Event::ToggleAdvanced(tag(sender)));
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

    fn send(&self, event: Event) {
        if let Some(handler) = self.ivars().handler.get() {
            handler.handle(event);
        }
    }

    /// Makes `control` call `action` of this target, tagged with `tag`.
    pub fn wire(&self, control: &NSControl, action: Sel, tag: isize) {
        // SAFETY: the target lives as long as the window that holds the control.
        unsafe { control.setTarget(Some(self as &AnyObject)) };
        // SAFETY: `action` is a method of this target taking the sender.
        unsafe { control.setAction(Some(action)) };
        control.setTag(tag);
    }
}

/// The selectors of the target's methods, for [`FormTarget::wire`].
pub mod actions {
    use super::{sel, Sel};

    pub fn changed() -> Sel {
        sel!(changed:)
    }
    pub fn stepped() -> Sel {
        sel!(stepped:)
    }
    pub fn play() -> Sel {
        sel!(play:)
    }
    pub fn choose() -> Sel {
        sel!(choose:)
    }
    pub fn reveal() -> Sel {
        sel!(reveal:)
    }
    pub fn add_example() -> Sel {
        sel!(addExample:)
    }
    pub fn remove_example() -> Sel {
        sel!(removeExample:)
    }
    pub fn toggle_advanced() -> Sel {
        sel!(toggleAdvanced:)
    }
    pub fn save() -> Sel {
        sel!(save:)
    }
    pub fn revert() -> Sel {
        sel!(revert:)
    }
    pub fn open_logs() -> Sel {
        sel!(openLogs:)
    }
    pub fn login_toggled() -> Sel {
        sel!(loginToggled:)
    }
    pub fn open_login_items() -> Sel {
        sel!(openLoginItems:)
    }
    pub fn download_models() -> Sel {
        sel!(downloadModels:)
    }
    pub fn show_models() -> Sel {
        sel!(showModels:)
    }
}

fn tag(sender: Option<&AnyObject>) -> isize {
    sender
        .and_then(|sender| sender.downcast_ref::<NSControl>())
        .map_or(-1, NSControl::tag)
}
