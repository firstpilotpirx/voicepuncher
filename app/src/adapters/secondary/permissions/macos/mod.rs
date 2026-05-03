use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait};

use crate::core::application::permission_probe::PermissionProbe;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
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
        let trusted = unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) };
        if !trusted {
            return Err(anyhow!("Accessibility permission not granted"));
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
