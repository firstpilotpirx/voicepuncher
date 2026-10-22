use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass};
use objc2_app_kit::{
    NSApplicationDidChangeScreenParametersNotification, NSWorkspace, NSWorkspaceDidWakeNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol};

pub(super) struct RebuildObserverIvars {
    handler: Arc<dyn Fn() + Send + Sync + 'static>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "VPRebuildObserver"]
    #[ivars = RebuildObserverIvars]
    pub(super) struct RebuildObserver;

    unsafe impl NSObjectProtocol for RebuildObserver {}

    impl RebuildObserver {
        #[unsafe(method(onEvent:))]
        fn on_event(&self, _notif: &NSNotification) {
            (self.ivars().handler)();
        }
    }
);

impl RebuildObserver {
    fn new(handler: Arc<dyn Fn() + Send + Sync + 'static>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(RebuildObserverIvars { handler });
        // SAFETY: `this` is a fresh allocation with its ivars set; NSObject's init takes it.
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) struct ScreenWatcher {
    observer: Retained<RebuildObserver>,
}

impl ScreenWatcher {
    pub(super) fn install(handler: Arc<dyn Fn() + Send + Sync + 'static>) -> Self {
        let observer = RebuildObserver::new(handler);

        let (on_event, observed) = (sel!(onEvent:), observer.as_ref() as &AnyObject);
        let center = NSNotificationCenter::defaultCenter();
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        let screens = unsafe { NSApplicationDidChangeScreenParametersNotification };
        // SAFETY: the watcher holds the observer and removes it in Drop, so it outlives its registration.
        unsafe {
            center.addObserver_selector_name_object(observed, on_event, Some(screens), None);
        }

        let wake_center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: an immutable framework constant, set before any Rust code runs.
        let did_wake = unsafe { NSWorkspaceDidWakeNotification };
        // SAFETY: as above: Drop removes the observer before the watcher releases it.
        unsafe {
            wake_center.addObserver_selector_name_object(observed, on_event, Some(did_wake), None);
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
