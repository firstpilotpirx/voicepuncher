use std::ffi::c_void;

use anyhow::{anyhow, Result};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};

use crate::core::application::focus_probe::FocusProbe;
use crate::core::domain::FocusedElement;

type AXUIElementRef = *const c_void;
type AXError = i32;

const AX_SUCCESS: AXError = 0;
const AX_NO_VALUE: AXError = -25212;
/// Longest a single Accessibility call may wait for a busy app before it fails.
const MESSAGING_TIMEOUT_SECS: f32 = 0.2;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementIsAttributeSettable(
        element: AXUIElementRef,
        attribute: CFStringRef,
        settable: *mut u8,
    ) -> AXError;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout_secs: f32) -> AXError;
}

/// Reads the focused element through the macOS Accessibility API.
pub struct MacOsFocusProbe;

impl MacOsFocusProbe {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsFocusProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusProbe for MacOsFocusProbe {
    fn focused_element(&self) -> Result<Option<FocusedElement>> {
        // SAFETY: every AX call gets a live element and an out-pointer it may fill; each
        // returned object is owned (create/copy rule) and released by its CFType wrapper.
        unsafe {
            let system = CFType::wrap_under_create_rule(AXUIElementCreateSystemWide());
            AXUIElementSetMessagingTimeout(system.as_CFTypeRef(), MESSAGING_TIMEOUT_SECS);
            let Some(focused) = copy_attribute(&system, "AXFocusedUIElement")? else {
                return Ok(None);
            };
            let role = copy_attribute(&focused, "AXRole")?
                .and_then(|role| role.downcast::<CFString>())
                .map(|role| role.to_string())
                .unwrap_or_else(|| "(no role)".to_owned());
            let has_text_selection =
                matches!(copy_attribute(&focused, "AXSelectedTextRange"), Ok(Some(_)));
            Ok(Some(FocusedElement {
                role,
                has_text_selection,
                value_settable: is_settable(&focused, "AXValue"),
            }))
        }
    }
}

/// `Ok(None)` when the element has no value for the attribute.
unsafe fn copy_attribute(element: &CFType, attribute: &str) -> Result<Option<CFType>> {
    let name = CFString::new(attribute);
    let mut value: CFTypeRef = std::ptr::null();
    match AXUIElementCopyAttributeValue(
        element.as_CFTypeRef(),
        name.as_concrete_TypeRef(),
        &mut value,
    ) {
        AX_SUCCESS if !value.is_null() => Ok(Some(CFType::wrap_under_create_rule(value))),
        AX_SUCCESS | AX_NO_VALUE => Ok(None),
        code => Err(anyhow!("AX error {code} reading {attribute}")),
    }
}

unsafe fn is_settable(element: &CFType, attribute: &str) -> bool {
    let name = CFString::new(attribute);
    let mut settable: u8 = 0;
    AXUIElementIsAttributeSettable(
        element.as_CFTypeRef(),
        name.as_concrete_TypeRef(),
        &mut settable,
    ) == AX_SUCCESS
        && settable != 0
}
