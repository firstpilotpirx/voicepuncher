//! Whether the login item started the app: the open-application Apple event of the launch
//! carries `keyAELaunchedAsLogInItem` as its property when it did.

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::{msg_send, MainThreadMarker};
use objc2_app_kit::NSApplicationDidFinishLaunchingNotification;
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter,
};

/// `kAEOpenApplication`: the event of a plain launch.
const OPEN_APPLICATION: u32 = u32::from_be_bytes(*b"oapp");
/// `keyAEPropData`: the parameter that holds how the app was opened.
const PROPERTY: u32 = u32::from_be_bytes(*b"prdt");
/// `keyAELaunchedAsLogInItem`: the property of a launch by the login item.
const LAUNCHED_AS_LOGIN_ITEM: u32 = u32::from_be_bytes(*b"lgit");

/// The log line of a launch by the login item.
pub const LAUNCHED_AT_LOGIN: &str = "launched at login";

/// The log line for the launch whose Apple event has the id `event_id` and the property
/// `property`; `None` for any launch the login item did not start, or with no event.
pub fn launch_line(event_id: Option<u32>, property: Option<u32>) -> Option<&'static str> {
    (event_id == Some(OPEN_APPLICATION) && property == Some(LAUNCHED_AS_LOGIN_ITEM))
        .then_some(LAUNCHED_AT_LOGIN)
}

/// Logs [`LAUNCHED_AT_LOGIN`] once the app finished launching, when the login item started
/// it; call before `NSApplication::run`.
pub fn log_launch_origin(_mtm: MainThreadMarker) {
    let block = RcBlock::new(|_: NonNull<NSNotification>| {
        if let Some(line) = current_launch_line() {
            log::info!("{line}");
        }
    });
    // SAFETY: the block captures nothing and runs on the posting (main) thread; the
    // observer stays registered for the life of the process, which is what we want.
    let observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSApplicationDidFinishLaunchingNotification),
            None,
            None,
            &block,
        )
    };
    std::mem::forget(observer);
}

/// The launch line from the Apple event being handled, which is the open-application
/// event while the app finishes launching.
fn current_launch_line() -> Option<&'static str> {
    let event = NSAppleEventManager::sharedAppleEventManager().currentAppleEvent()?;
    // SAFETY: `eventID` returns the event's four-char code; `paramDescriptorForKeyword:`
    // takes one and returns an autoreleased descriptor or nil. Both are plain
    // NSAppleEventDescriptor methods, called through msg_send so the typed bindings'
    // CoreServices feature is not needed.
    let event_id: u32 = unsafe { msg_send![&*event, eventID] };
    let property: Option<Retained<NSAppleEventDescriptor>> =
        unsafe { msg_send![&*event, paramDescriptorForKeyword: PROPERTY] };
    launch_line(
        Some(event_id),
        property.map(|descriptor| descriptor.enumCodeValue()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_say_launched_at_login_for_the_login_item_event() {
        assert_eq!(
            launch_line(Some(OPEN_APPLICATION), Some(LAUNCHED_AS_LOGIN_ITEM)),
            Some(LAUNCHED_AT_LOGIN)
        );
    }

    #[test]
    fn should_say_nothing_for_an_open_application_event_without_the_property() {
        assert_eq!(launch_line(Some(OPEN_APPLICATION), None), None);
    }

    #[test]
    fn should_say_nothing_for_another_property() {
        let service = u32::from_be_bytes(*b"lgsi");
        assert_eq!(launch_line(Some(OPEN_APPLICATION), Some(service)), None);
    }

    #[test]
    fn should_say_nothing_for_another_event() {
        let open_documents = u32::from_be_bytes(*b"odoc");
        assert_eq!(
            launch_line(Some(open_documents), Some(LAUNCHED_AS_LOGIN_ITEM)),
            None
        );
    }

    #[test]
    fn should_say_nothing_without_an_event() {
        assert_eq!(launch_line(None, None), None);
    }
}
