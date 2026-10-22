//! A quit from the system — logout, shutdown, Activity Monitor's Quit, any quit Apple Event —
//! takes the menu's Quit: the runtime and the services the app started stop before it ends.

use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSApplicationTerminateReply};
use objc2_foundation::{NSObject, NSObjectProtocol};

use super::actions::MenuActions;

/// What the app logs when the system tells it to quit.
pub const SYSTEM_QUIT_NOTE: &str = "quit: asked by the system";

pub struct SystemQuitIvars {
    actions: Rc<dyn MenuActions>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "VPAppDelegate"]
    #[ivars = SystemQuitIvars]
    pub struct SystemQuit;

    unsafe impl NSObjectProtocol for SystemQuit {}

    unsafe impl NSApplicationDelegate for SystemQuit {
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _sender: &NSApplication) -> NSApplicationTerminateReply {
            log::info!("{SYSTEM_QUIT_NOTE}");
            // Quit stops the runtime and ends the process; it returns only when it cannot
            self.ivars().actions.quit();
            NSApplicationTerminateReply::TerminateNow
        }
    }
);

impl SystemQuit {
    /// Makes `actions`' Quit the answer to every quit from the system; the app holds its
    /// delegate weakly, so the caller keeps the returned object for the life of the run loop.
    pub fn install(mtm: MainThreadMarker, actions: Rc<dyn MenuActions>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SystemQuitIvars { actions });
        // SAFETY: `NSObject`'s `init` on a freshly allocated object.
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        NSApplication::sharedApplication(mtm).setDelegate(Some(ProtocolObject::from_ref(&*this)));
        this
    }
}
