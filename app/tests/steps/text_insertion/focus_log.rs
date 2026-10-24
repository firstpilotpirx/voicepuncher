//! The focus decision the pipeline logs for every dictation it delivers while the focus wait
//! is on, read from the logger the scenarios share.

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use cucumber::{given, then};

use super::REACTION;
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

const WAITS: &str = "→ wait";

/// The focus lines logged since the scenario turned the focus wait on.
fn logged_since(world: &IntegrationWorld) -> Result<Vec<String>> {
    let mark = world
        .extra
        .insertion
        .focus_log_mark
        .context("the focus wait was not turned on")?;
    Ok(pipeline_log().focus.lock().unwrap()[mark..].to_vec())
}

/// Waits until the pipeline logged a focus line that `wanted` accepts.
fn wait_for_line(world: &IntegrationWorld, wanted: impl Fn(&str) -> bool) -> Result<()> {
    let deadline = Instant::now() + REACTION;
    loop {
        let lines = logged_since(world)?;
        if lines.iter().any(|line| wanted(line)) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("no such focus line in the log: {lines:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[given("the focus wait is on")]
fn focus_wait_on(world: &mut IntegrationWorld) {
    // the scenarios' wait time is not zero, so the wait is on; the log is read from here on
    world.extra.insertion.focus_log_mark = Some(pipeline_log().focus.lock().unwrap().len());
}

#[then("the log names the button's kind and says the app waits for focus")]
fn names_button(world: &mut IntegrationWorld) -> Result<()> {
    wait_for_line(world, |line| {
        line.starts_with("focus: AXButton ") && line.contains(WAITS)
    })
}

#[then(
    "the log names the page's kind, says it lies in web content and says the app waits for focus"
)]
fn names_web_page(world: &mut IntegrationWorld) -> Result<()> {
    wait_for_line(world, |line| {
        line.starts_with("focus: AXWebArea ") && line.contains("+web") && line.contains(WAITS)
    })
}
