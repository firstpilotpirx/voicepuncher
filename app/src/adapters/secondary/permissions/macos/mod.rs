use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait};

use crate::core::application::permission_probe::PermissionProbe;

type CGEventTapProxy = *mut std::ffi::c_void;
type CGEventRef = *mut std::ffi::c_void;
type CFMachPortRef = *mut std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
    fn CGEventTapCreate(
        tap: std::ffi::c_uint,
        place: std::ffi::c_uint,
        options: std::ffi::c_uint,
        eventsOfInterest: u64,
        callback: Option<
            unsafe extern "C" fn(
                CGEventTapProxy,
                std::ffi::c_uint,
                CGEventRef,
                *mut std::ffi::c_void,
            ) -> CGEventRef,
        >,
        userInfo: *mut std::ffi::c_void,
    ) -> CFMachPortRef;
    fn CFMachPortInvalidate(port: CFMachPortRef);
    fn CFRelease(cf: *const std::ffi::c_void);
}

/// Whether macOS trusts this process for accessibility, so the events it posts arrive.
pub fn process_is_trusted_for_accessibility() -> bool {
    // SAFETY: a null options dictionary asks without prompting the user.
    unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) }
}

pub struct MacOsPermissionProbe;

impl MacOsPermissionProbe {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsPermissionProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionProbe for MacOsPermissionProbe {
    fn check_microphone(&self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("no default input device"))?;
        let config = device.default_input_config()?;
        let stream = device.build_input_stream(
            &config.config(),
            |_data: &[f32], _: &cpal::InputCallbackInfo| {},
            |_err| {},
            None,
        );
        match stream {
            Ok(_stream) => Ok(()),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    fn check_accessibility(&self) -> Result<()> {
        if !process_is_trusted_for_accessibility() {
            return Err(anyhow!("Accessibility permission not granted"));
        }
        Ok(())
    }

    fn check_input_monitoring(&self) -> Result<()> {
        const K_CG_HID_EVENT_TAP: std::ffi::c_uint = 0;
        const K_CG_HEAD_INSERT_EVENT_TAP: std::ffi::c_uint = 0;
        const K_CG_EVENT_TAP_OPTION_DEFAULT: std::ffi::c_uint = 0;
        const K_CG_EVENT_KEY_DOWN: std::ffi::c_uint = 10;

        unsafe extern "C" fn dummy_callback(
            _proxy: CGEventTapProxy,
            _event_type: std::ffi::c_uint,
            _event: CGEventRef,
            _user_info: *mut std::ffi::c_void,
        ) -> CGEventRef {
            _event
        }

        let tap = unsafe {
            CGEventTapCreate(
                K_CG_HID_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                1u64 << K_CG_EVENT_KEY_DOWN,
                Some(dummy_callback),
                std::ptr::null_mut(),
            )
        };

        if tap.is_null() {
            return Err(anyhow!("Input Monitoring permission not granted"));
        }

        unsafe {
            CFMachPortInvalidate(tap);
            CFRelease(tap as *const std::ffi::c_void);
        }

        Ok(())
    }

    fn parent_process_name(&self) -> Result<String> {
        let Ok(output) = std::process::Command::new("sh")
            .args(["-c", "ps -o comm= -p $PPID"])
            .output()
        else {
            return Ok("unknown".into());
        };
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if name.is_empty() {
            "unknown".into()
        } else {
            name
        })
    }
}
