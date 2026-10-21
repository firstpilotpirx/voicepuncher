//! The permissions on macOS: status calls that open no device, the system's requests, and the
//! System Settings pages.

mod responsible_app;

use std::ffi::c_void;

use block2::RcBlock;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use objc2::runtime::{AnyClass, Bool};
use objc2::{class, msg_send};
use objc2_foundation::{NSBundle, NSString};

use crate::core::application::permission_probe::PermissionProbe;
use crate::core::domain::{Permission, PermissionStatus};

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOHIDCheckAccess(request: u32) -> u32;
    fn IOHIDRequestAccess(request: u32) -> bool;
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVMediaTypeAudio: &'static NSString;
}

/// `kIOHIDRequestTypeListenEvent`: Input Monitoring.
const IOHID_REQUEST_LISTEN_EVENT: u32 = 1;
/// `kIOHIDAccessTypeGranted`, `kIOHIDAccessTypeDenied`; anything else is unknown.
const IOHID_ACCESS_GRANTED: u32 = 0;
const IOHID_ACCESS_DENIED: u32 = 1;
/// `AVAuthorizationStatus`: NotDetermined 0, Restricted 1, Denied 2, Authorized 3.
const AV_AUTHORIZATION_NOT_DETERMINED: isize = 0;
const AV_AUTHORIZATION_AUTHORIZED: isize = 3;

const PRIVACY_SETTINGS: &str = "x-apple.systempreferences:com.apple.preference.security";

/// Whether macOS trusts this process for accessibility, so the events it posts arrive.
pub fn process_is_trusted_for_accessibility() -> bool {
    // SAFETY: a null options dictionary asks without prompting the user.
    unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) }
}

#[derive(Default)]
pub struct MacOsPermissionProbe;

impl MacOsPermissionProbe {
    pub fn new() -> Self {
        Self
    }
}

impl PermissionProbe for MacOsPermissionProbe {
    fn status(&self, permission: Permission) -> PermissionStatus {
        match permission {
            Permission::Microphone => microphone_status(),
            Permission::Accessibility => accessibility_status(),
            Permission::InputMonitoring => input_monitoring_status(),
        }
    }

    fn request(&self, permission: Permission) {
        match permission {
            Permission::Microphone => request_microphone(),
            Permission::Accessibility => request_accessibility(),
            Permission::InputMonitoring => {
                // SAFETY: a plain C call; the answer shows in a later status call.
                unsafe { IOHIDRequestAccess(IOHID_REQUEST_LISTEN_EVENT) };
            }
        }
    }

    fn settings_url(&self, permission: Permission) -> String {
        format!("{PRIVACY_SETTINGS}?{}", settings_anchor(permission))
    }

    fn responsible_app(&self) -> Option<String> {
        if runs_from_bundle() {
            return None;
        }
        Some(responsible_app::parent_application())
    }
}

/// The anchor of a permission's page in Privacy & Security.
pub fn settings_anchor(permission: Permission) -> &'static str {
    match permission {
        Permission::Microphone => "Privacy_Microphone",
        Permission::Accessibility => "Privacy_Accessibility",
        Permission::InputMonitoring => "Privacy_ListenEvent",
    }
}

fn capture_device() -> &'static AnyClass {
    class!(AVCaptureDevice)
}

fn microphone_status() -> PermissionStatus {
    // SAFETY: `authorizationStatusForMediaType:` takes a media type constant and returns an
    // `NSInteger`; it opens no device.
    let status: isize =
        unsafe { msg_send![capture_device(), authorizationStatusForMediaType: AVMediaTypeAudio] };
    match status {
        AV_AUTHORIZATION_AUTHORIZED => PermissionStatus::Granted,
        AV_AUTHORIZATION_NOT_DETERMINED => PermissionStatus::NotDetermined,
        _ => PermissionStatus::Denied,
    }
}

fn request_microphone() {
    // WHY: the answer arrives on an arbitrary thread; the status poll picks it up.
    let completion = RcBlock::new(|_granted: Bool| {});
    // SAFETY: the block is copied by AVFoundation and lives until it has run.
    unsafe {
        let _: () = msg_send![
            capture_device(),
            requestAccessForMediaType: AVMediaTypeAudio,
            completionHandler: &*completion
        ];
    }
}

fn accessibility_status() -> PermissionStatus {
    if process_is_trusted_for_accessibility() {
        PermissionStatus::Granted
    } else {
        // WHY: macOS tells no "never asked" for Accessibility; a request is always offered
        // through System Settings, so the status is Not granted
        PermissionStatus::Denied
    }
}

fn request_accessibility() {
    // SAFETY: the key is a CFString constant of ApplicationServices; the dictionary lives
    // across the call.
    let prompt = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let options = CFDictionary::from_CFType_pairs(&[(prompt, CFBoolean::true_value())]);
    // SAFETY: a valid options dictionary; the call shows the system dialog once per process.
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef().cast()) };
}

fn input_monitoring_status() -> PermissionStatus {
    // SAFETY: a plain C call that only reads the TCC database.
    match unsafe { IOHIDCheckAccess(IOHID_REQUEST_LISTEN_EVENT) } {
        IOHID_ACCESS_GRANTED => PermissionStatus::Granted,
        IOHID_ACCESS_DENIED => PermissionStatus::Denied,
        _ => PermissionStatus::NotDetermined,
    }
}

/// A bundle has an identifier; a bare binary run from a terminal has none.
fn runs_from_bundle() -> bool {
    NSBundle::mainBundle().bundleIdentifier().is_some()
}
