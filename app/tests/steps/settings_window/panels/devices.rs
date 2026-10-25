//! The input device a recording opens for the configured microphone: the app's choice among
//! the Mac's device names, and the real system default when the named one is not connected.

use cpal::traits::{DeviceTrait, HostTrait};
use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::adapters::secondary::recording::cpal::devices::{numbered, open, position};

use crate::IntegrationWorld;

/// The Mac's devices in the scenario with a named microphone: the default first.
const CONNECTED: [&str; 2] = ["MacBook Pro Microphone", "USB Mic"];
const NAMED: &str = "USB Mic";
const NOT_CONNECTED: &str = "No Such Microphone";

/// What a scenario of the chosen microphone shares between its steps.
#[derive(Debug, Default)]
pub struct MicrophoneRun {
    devices: Vec<String>,
    configured: String,
    /// The connected device the recording opened, by its place among `devices`.
    chosen: Option<usize>,
    /// The device opened instead, or why none opened.
    fallback: Option<Result<String, String>>,
}

fn microphone(world: &mut IntegrationWorld) -> &mut MicrophoneRun {
    world
        .extra
        .ui
        .microphone
        .get_or_insert_with(MicrophoneRun::default)
}

fn device_name(device: &cpal::Device) -> String {
    device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_default()
}

#[given("the configuration names a connected input device that is not the system default")]
fn names_connected_device(world: &mut IntegrationWorld) {
    let run = microphone(world);
    run.devices = CONNECTED.map(str::to_owned).to_vec();
    NAMED.clone_into(&mut run.configured);
}

#[given("the configuration names an input device that is not connected")]
fn names_missing_device(world: &mut IntegrationWorld) {
    let run = microphone(world);
    run.devices = CONNECTED.map(str::to_owned).to_vec();
    NOT_CONNECTED.clone_into(&mut run.configured);
}

/// The recording's device: the named one when it is among the devices, else what the app
/// opens on this Mac.
#[when("the recording opens its input device")]
fn recording_opens_device(world: &mut IntegrationWorld) {
    crate::steps::voice_control::pipeline_log();
    let run = microphone(world);
    run.chosen = position(&numbered(&run.devices), &run.configured);
    if run.chosen.is_none() {
        let opened = open(&run.configured).map(|device| device_name(&device));
        run.fallback = Some(opened.map_err(|error| error.to_string()));
    }
}

#[then("the recording comes from the named device")]
fn from_named_device(world: &mut IntegrationWorld) -> Result<()> {
    let run = microphone(world);
    let chosen = run.chosen.map(|place| run.devices[place].clone());
    if chosen.as_deref() != Some(NAMED) || run.fallback.is_some() {
        bail!("the recording opened {chosen:?}, else {:?}", run.fallback);
    }
    Ok(())
}

#[then("the recording comes from the system default device")]
fn from_default_device(world: &mut IntegrationWorld) -> Result<()> {
    let opened = microphone(world)
        .fallback
        .clone()
        .context("no device was opened")?;
    let default = cpal::default_host().default_input_device();
    match (default.as_ref().map(device_name), opened) {
        (Some(default), Ok(opened)) if default == opened => Ok(()),
        (None, Err(error)) if error.contains("no default input device") => Ok(()),
        (default, opened) => bail!("the default is {default:?}, the recording opened {opened:?}"),
    }
}

#[then("the log names the missing device")]
fn log_names_missing_device(_world: &mut IntegrationWorld) -> Result<()> {
    let warnings = crate::steps::voice_control::pipeline_log()
        .warnings
        .lock()
        .unwrap()
        .clone();
    if !warnings
        .iter()
        .any(|(_, line)| line.contains(NOT_CONNECTED))
    {
        bail!("no warning names {NOT_CONNECTED:?}: {warnings:?}");
    }
    Ok(())
}
