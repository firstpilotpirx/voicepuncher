//! The app as a bare binary: this harness, started from a terminal, puts the app's overlay and
//! status item up as the app does and asks the system how it shows the process.

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::{NSDate, NSRunLoop};

use voicepuncher::adapters::primary::menubar::macos::indicator::StatusItemCell;
use voicepuncher::adapters::primary::menubar::macos::status_item::MacosStatusItem;
use voicepuncher::adapters::secondary::visualfeedback::macos::MacosVisualIndicator;

use super::items;
use crate::IntegrationWorld;

/// How long the main run loop gets to draw what was dispatched to it.
const DRAW: Duration = Duration::from_millis(300);
const NOTICE_LIFETIME: Duration = Duration::from_secs(1);

/// The overlay and the status item of the running app.
pub struct RunningApp {
    status_item: MacosStatusItem,
    _overlay: MacosVisualIndicator,
}

fn main_thread() -> Result<MainThreadMarker> {
    MainThreadMarker::new().context("the harness runs its steps on the main thread")
}

fn pump_run_loop() {
    let deadline = NSDate::dateWithTimeIntervalSinceNow(DRAW.as_secs_f64());
    NSRunLoop::currentRunLoop().runUntilDate(&deadline);
}

#[given("the app is started from a terminal")]
fn started_from_terminal(_world: &mut IntegrationWorld) {
    // this harness is a bare binary a terminal started, as the app is when run without its bundle
}

#[when("it is running")]
fn it_is_running(world: &mut IntegrationWorld) -> Result<()> {
    let mtm = main_thread()?;
    let overlay = MacosVisualIndicator::initialize(mtm, Arc::new(AtomicU32::new(0)));
    let status_item = MacosStatusItem::install(mtm);
    let cell = StatusItemCell::new(Box::new(status_item.icon()), NOTICE_LIFETIME);
    status_item.connect(cell);
    pump_run_loop();
    items(world).running = Some(RunningApp { status_item, _overlay: overlay });
    Ok(())
}

#[then("the Dock shows no icon for it and the status item is in the menu bar")]
fn no_dock_icon(world: &mut IntegrationWorld) -> Result<()> {
    let mtm = main_thread()?;
    let policy = NSApplication::sharedApplication(mtm).activationPolicy();
    if policy != NSApplicationActivationPolicy::Accessory {
        bail!("the process shows in the Dock: activation policy {policy:?}");
    }
    let running = items(world)
        .running
        .as_ref()
        .context("the app is not running")?;
    if running.status_item.shown_symbol(mtm).is_none() {
        bail!("the status item shows no icon");
    }
    Ok(())
}
