//! The input device a recording opens: the one the configuration names, else the
//! system default.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Device;

use crate::core::application::InputDevices;

/// The `input_device` value that names the system default input device.
pub const DEFAULT_DEVICE: &str = "default";

/// The devices' raw names made unique: a repeated name gets ` #2`, ` #3` … from its
/// second device on, in the order the system lists them.
pub fn numbered(raw: &[String]) -> Vec<String> {
    raw.iter()
        .enumerate()
        .map(|(index, name)| {
            let earlier = raw[..index].iter().filter(|other| *other == name).count();
            match earlier {
                0 => name.clone(),
                _ => format!("{name} #{}", earlier + 1),
            }
        })
        .collect()
}

/// The position in `names` (as [`numbered`] gives them) of the device `configured`
/// names; `None` for the system default and for a device that is not connected.
pub fn position(names: &[String], configured: &str) -> Option<usize> {
    if configured == DEFAULT_DEVICE {
        return None;
    }
    names.iter().position(|name| name == configured)
}

/// The input device `configured` names, or the system default when it names the
/// default or a device that is not connected (logged).
pub fn open(configured: &str) -> Result<Device> {
    let host = cpal::default_host();
    let devices: Vec<Device> = host.input_devices()?.collect();
    let names = numbered(&devices.iter().map(raw_name).collect::<Vec<_>>());
    if let Some(index) = position(&names, configured) {
        return Ok(devices[index].clone());
    }
    if configured != DEFAULT_DEVICE {
        log::warn!(
            "input device \"{configured}\" is not connected; recording from the system default"
        );
    }
    host.default_input_device()
        .ok_or_else(|| anyhow!("no default input device available"))
}

/// The Mac's input devices through cpal.
pub struct CpalInputDevices;

impl InputDevices for CpalInputDevices {
    fn names(&self) -> Vec<String> {
        let raw: Vec<String> = cpal::default_host()
            .input_devices()
            .map(|devices| devices.map(|device| raw_name(&device)).collect())
            .unwrap_or_default();
        numbered(&raw)
    }
}

fn raw_name(device: &Device) -> String {
    device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
