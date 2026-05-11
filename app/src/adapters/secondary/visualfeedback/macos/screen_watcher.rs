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
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) struct ScreenWatcher {
    observer: Retained<RebuildObserver>,
}

impl ScreenWatcher {
    pub(super) fn install(handler: Arc<dyn Fn() + Send + Sync + 'static>) -> Self {
        let observer = RebuildObserver::new(handler);

        let default_center = NSNotificationCenter::defaultCenter();
        unsafe {
            default_center.addObserver_selector_name_object(
                observer.as_ref() as &AnyObject,
                sel!(onEvent:),
                Some(NSApplicationDidChangeScreenParametersNotification),
                None,
            );
        }

        let workspace_center = NSWorkspace::sharedWorkspace().notificationCenter();
        unsafe {
            workspace_center.addObserver_selector_name_object(
                observer.as_ref() as &AnyObject,
                sel!(onEvent:),
                Some(NSWorkspaceDidWakeNotification),
                None,
            );
        }

        Self { observer }
    }
}

impl Drop for ScreenWatcher {
    fn drop(&mut self) {
        let observer_ref = self.observer.as_ref() as &AnyObject;

        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(observer_ref);
        }

        let workspace_center = NSWorkspace::sharedWorkspace().notificationCenter();
        unsafe {
            workspace_center.removeObserver(observer_ref);
        }
    }
}
