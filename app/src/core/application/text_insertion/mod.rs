use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::core::application::{FocusProbe, PipelineInput, TextPaster, VisualIndicator};
use crate::core::domain::{CleanedText, PttSignal, TextFocus};

/// How long the focus wait lasts and how often it re-checks the focus. A zero timeout
/// turns the wait off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusWaitCfg {
    pub timeout: Duration,
    pub poll_interval: Duration,
}

impl FocusWaitCfg {
    pub const OFF: Self = Self { timeout: Duration::ZERO, poll_interval: Duration::ZERO };
}

/// What tells whether text input has focus, none without a probe, and how long to wait for it.
#[derive(Clone, Copy)]
pub struct FocusWait<'a> {
    pub probe: Option<&'a dyn FocusProbe>,
    pub cfg: FocusWaitCfg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insertion {
    Pasted,
    PastedAfterWait,
    LeftInClipboard(WaitEnd),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitEnd {
    TimedOut,
    HotkeyPressed,
}

/// Delivers the final text: pastes at once unless no text input is surely focused, then
/// waits for text input focus; a wait that ends without one leaves the text in the clipboard.
///
/// The wait ends early when `hotkey_pressed` reports a hotkey press.
pub fn insert_text(
    text: &CleanedText,
    paster: &mut dyn TextPaster,
    wait: FocusWait<'_>,
    indicator: &dyn VisualIndicator,
    hotkey_pressed: &mut dyn FnMut() -> bool,
) -> Result<Insertion> {
    let FocusWait { probe, cfg } = wait;
    let probe = match probe {
        Some(probe) if !cfg.timeout.is_zero() => probe,
        _ => {
            paster.paste(text)?;
            return Ok(Insertion::Pasted);
        }
    };
    let focus = text_focus(probe);
    log::info!("{}", decision_line(&focus, cfg.timeout));
    if !focus.waits() {
        paster.paste(text)?;
        return Ok(Insertion::Pasted);
    }
    indicator.show_waiting_for_focus();
    paste_once_focused(text, paster, probe, cfg, hotkey_pressed)
}

/// Waits until a text field has focus, then pastes; leaves the text in the clipboard when the
/// wait times out or the hotkey is pressed.
fn paste_once_focused(
    text: &CleanedText,
    paster: &mut dyn TextPaster,
    probe: &dyn FocusProbe,
    cfg: FocusWaitCfg,
    hotkey_pressed: &mut dyn FnMut() -> bool,
) -> Result<Insertion> {
    let started = Instant::now();
    loop {
        std::thread::sleep(cfg.poll_interval);
        if hotkey_pressed() {
            return leave_in_clipboard(text, paster, WaitEnd::HotkeyPressed, started);
        }
        let focus = text_focus(probe);
        if !focus.waits() {
            log::info!(
                "focus: {} after {}ms → paste",
                focus.seen(),
                started.elapsed().as_millis()
            );
            paster.paste(text)?;
            return Ok(Insertion::PastedAfterWait);
        }
        if started.elapsed() >= cfg.timeout {
            return leave_in_clipboard(text, paster, WaitEnd::TimedOut, started);
        }
    }
}

/// The log line naming what has focus and whether the text is pasted or held back.
fn decision_line(focus: &TextFocus, timeout: Duration) -> String {
    match focus {
        TextFocus::Paste { seen } => format!("focus: {seen} → paste"),
        TextFocus::Wait { seen } => format!("focus: {seen} → wait (up to {}s)", timeout.as_secs()),
    }
}

fn text_focus(probe: &dyn FocusProbe) -> TextFocus {
    let probed = probe.focused_element();
    TextFocus::of(probed.as_ref().map(Option::as_ref))
}

/// Reads the inputs that arrived during a focus wait: a hotkey press or cancel is
/// consumed and reported; every other input is kept in `pending`, in order, for later.
pub fn hotkey_pressed_during_wait(
    inputs: &Receiver<PipelineInput>,
    pending: &mut VecDeque<PipelineInput>,
) -> bool {
    let mut pressed = false;
    while let Ok(input) = inputs.try_recv() {
        match input {
            PipelineInput::Hotkey(PttSignal::Pressed | PttSignal::Cancel) => pressed = true,
            other => pending.push_back(other),
        }
    }
    pressed
}

fn leave_in_clipboard(
    text: &CleanedText,
    paster: &mut dyn TextPaster,
    reason: WaitEnd,
    started: Instant,
) -> Result<Insertion> {
    paster.leave_in_clipboard(text)?;
    let reason_text = match reason {
        WaitEnd::TimedOut => "timed out",
        WaitEnd::HotkeyPressed => "hotkey pressed",
    };
    log::info!(
        "focus wait ended ({reason_text}) after {}ms: text left in clipboard",
        started.elapsed().as_millis()
    );
    Ok(Insertion::LeftInClipboard(reason))
}

#[cfg(test)]
mod tests;
