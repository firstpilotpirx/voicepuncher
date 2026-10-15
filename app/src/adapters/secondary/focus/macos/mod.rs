use std::ffi::c_void;

use anyhow::{anyhow, Result};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::{CFString, CFStringRef};

use crate::core::application::focus_probe::FocusProbe;
use crate::core::domain::FocusedElement;

type AXUIElementRef = *const c_void;
type AXError = i32;

const AX_SUCCESS: AXError = 0;
const AX_NO_VALUE: AXError = -25212;
/// Longest a single Accessibility call may wait for a busy app before it fails.
const MESSAGING_TIMEOUT_SECS: f32 = 0.2;
/// Most ancestors looked at when telling whether the focused element lies in a web page.
const MAX_ANCESTORS: usize = 64;

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
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
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
            expose_web_content(&system);
            let Some(focused) = copy_attribute(&system, "AXFocusedUIElement")? else {
                return Ok(None);
            };
            let role = role_of(&focused)?.unwrap_or_else(|| "(no role)".to_owned());
            let has_text_selection =
                matches!(copy_attribute(&focused, "AXSelectedTextRange"), Ok(Some(_)));
            let in_web_content = role == WEB_AREA_ROLE || has_web_area_ancestor(&focused);
            Ok(Some(FocusedElement {
                role,
                has_text_selection,
                value_settable: is_settable(&focused, "AXValue"),
                in_web_content,
            }))
        }
    }
}

const WEB_AREA_ROLE: &str = "AXWebArea";

/// Asks the focused app to build its web accessibility tree, so Chromium and Electron report
/// a text input on a page as a text input. Other apps reject the attribute; that is ignored.
/// Setting it again is a no-op, so every probe does it and apps started later are covered.
unsafe fn expose_web_content(system: &CFType) {
    if let Ok(Some(app)) = copy_attribute(system, "AXFocusedApplication") {
        let name = CFString::new("AXManualAccessibility");
        AXUIElementSetAttributeValue(
            app.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
    }
}

/// Walks `AXParent` up to a web page. An error ends the walk with `false`: an unknown
/// position keeps the rule for elements outside web content, which pastes at once.
unsafe fn has_web_area_ancestor(element: &CFType) -> bool {
    let mut current = element.clone();
    for _ in 0..MAX_ANCESTORS {
        let Ok(Some(parent)) = copy_attribute(&current, "AXParent") else {
            return false;
        };
        match role_of(&parent).ok().flatten().as_deref() {
            Some(WEB_AREA_ROLE) => return true,
            Some("AXWindow" | "AXApplication") | None => return false,
            Some(_) => current = parent,
        }
    }
    false
}

unsafe fn role_of(element: &CFType) -> Result<Option<String>> {
    Ok(copy_attribute(element, "AXRole")?
        .and_then(|role| role.downcast::<CFString>())
        .map(|role| role.to_string()))
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
