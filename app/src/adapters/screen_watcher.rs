//! Calls a handler when the screens change or the Mac wakes: the overlay rebuilds its windows;
//! and another when the user switches desktop: the overlay checks it is on the new one.

use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass};
use objc2_app_kit::{
    NSApplicationDidChangeScreenParametersNotification, NSWorkspace, NSWorkspaceDidWakeNotification,
};
use objc2_foundation::{
    ns_string, NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, NSString,
};

/// The name of the workspace's notification that the user switched desktop.
fn desktop_switch() -> &'static NSString {
    ns_string!("NSWorkspaceActiveSpaceDidChangeNotification")
}

/// What the watcher calls: on a screen change or a wake, and on a desktop switch.
pub(crate) struct WatchHandlers {
    pub(crate) screens: Arc<dyn Fn() + Send + Sync + 'static>,
    pub(crate) desktop: Arc<dyn Fn() + Send + Sync + 'static>,
}

pub(crate) struct RebuildObserverIvars {
    handlers: WatchHandlers,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "VPRebuildObserver"]
    #[ivars = RebuildObserverIvars]
    pub(crate) struct RebuildObserver;

    unsafe impl NSObjectProtocol for RebuildObserver {}

    impl RebuildObserver {
        #[unsafe(method(onEvent:))]
        fn on_event(&self, notif: &NSNotification) {
            let handlers = &self.ivars().handlers;
            if &*notif.name() == desktop_switch() {
                (handlers.desktop)();
            } else {
                (handlers.screens)();
            }
        }
    }
);

impl RebuildObserver {
    fn new(handlers: WatchHandlers) -> Retained<Self> {
        let this = Self::alloc().set_ivars(RebuildObserverIvars { handlers });
        // SAFETY: `this` is a fresh allocation with its ivars set; NSObject's init takes it.
        unsafe { msg_send![super(this), init] }
    }
}

pub(crate) struct ScreenWatcher {
    observer: Retained<RebuildObserver>,
}

impl ScreenWatcher {
    pub(crate) fn install(handlers: WatchHandlers) -> Self {
        let observer = RebuildObserver::new(handlers);

        let (on_event, listener) = (sel!(onEvent:), observer.as_ref() as &AnyObject);
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        let screens = unsafe { NSApplicationDidChangeScreenParametersNotification };
        // SAFETY: the watcher holds the observer and removes it in Drop, so it outlives its registration.
        unsafe {
            center.addObserver_selector_name_object(listener, on_event, Some(screens), None);
        }

        let wake_center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        let did_wake = unsafe { NSWorkspaceDidWakeNotification };
        for name in [did_wake, desktop_switch()] {
            // SAFETY: as above: Drop removes the observer before the watcher releases it.
            unsafe {
                wake_center.addObserver_selector_name_object(listener, on_event, Some(name), None);
            }
        }

        Self { observer }
    }
}

impl Drop for ScreenWatcher {
    fn drop(&mut self) {
        let observer_ref = self.observer.as_ref() as &AnyObject;

        // SAFETY: removes the observer `install` added; the watcher still holds it here.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(observer_ref) };

        let workspace_center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: as above.
        unsafe {
            workspace_center.removeObserver(observer_ref);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counter() -> (Arc<AtomicUsize>, Arc<dyn Fn() + Send + Sync + 'static>) {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        (
            count,
            Arc::new(move || {
                seen.fetch_add(1, Ordering::SeqCst);
            }),
        )
    }

    #[test]
    fn should_call_the_desktop_handler_alone_on_a_desktop_switch() {
        let (screens, on_screens) = counter();
        let (desktop, on_desktop) = counter();
        let _watcher =
            ScreenWatcher::install(WatchHandlers { screens: on_screens, desktop: on_desktop });

        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: a notification with no object; the watcher above outlives the post.
        unsafe { center.postNotificationName_object(desktop_switch(), None) };

        assert_eq!(desktop.load(Ordering::SeqCst), 1, "desktop handler calls");
        assert_eq!(screens.load(Ordering::SeqCst), 0, "screen handler calls");
    }
}
