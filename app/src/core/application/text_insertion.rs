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
    pub const OFF: FocusWaitCfg = FocusWaitCfg {
        timeout: Duration::ZERO,
        poll_interval: Duration::ZERO,
    };
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
/// The wait ends early when `hotkey_pressed` reports a hotkey press.
pub fn insert_text(
    text: &CleanedText,
    paster: &mut dyn TextPaster,
    probe: Option<&dyn FocusProbe>,
    indicator: &dyn VisualIndicator,
    cfg: FocusWaitCfg,
    hotkey_pressed: &mut dyn FnMut() -> bool,
) -> Result<Insertion> {
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
mod tests {
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::core::application::focus_probe::MockFocusProbe;
    use crate::core::application::text_paster::MockTextPaster;
    use crate::core::application::visual_indicator::MockVisualIndicator;
    use crate::core::domain::FocusedElement;

    const WAIT: FocusWaitCfg = FocusWaitCfg {
        timeout: Duration::from_millis(30),
        poll_interval: Duration::from_millis(1),
    };

    fn element(role: &str) -> FocusedElement {
        FocusedElement {
            role: role.to_owned(),
            has_text_selection: role == "AXTextField",
            value_settable: role == "AXTextField",
            in_web_content: false,
        }
    }

    fn text() -> CleanedText {
        CleanedText::new("hello".to_owned()).unwrap()
    }

    /// Answers `first` for the first `switch_after` checks, then `then`.
    fn probe(first: &'static str, switch_after: usize, then: &'static str) -> MockFocusProbe {
        let calls = Arc::new(Mutex::new(0usize));
        let mut probe = MockFocusProbe::new();
        probe.expect_focused_element().returning(move || {
            let mut calls = calls.lock().unwrap();
            let n = *calls;
            *calls += 1;
            Ok(Some(element(if n < switch_after { first } else { then })))
        });
        probe
    }

    fn pasting_once() -> MockTextPaster {
        let mut paster = MockTextPaster::new();
        paster.expect_paste().once().returning(|_| Ok(()));
        paster.expect_leave_in_clipboard().never();
        paster
    }

    fn leaving_in_clipboard() -> MockTextPaster {
        let mut paster = MockTextPaster::new();
        paster.expect_paste().never();
        paster
            .expect_leave_in_clipboard()
            .once()
            .returning(|_| Ok(()));
        paster
    }

    fn indicator(waiting_shown: usize) -> MockVisualIndicator {
        let mut indicator = MockVisualIndicator::new();
        indicator
            .expect_show_waiting_for_focus()
            .times(waiting_shown)
            .return_const(());
        indicator
    }

    #[test]
    fn should_log_the_focused_element_and_the_decision() {
        let button = element("AXButton");
        let field = element("AXTextField");
        assert_eq!(
            decision_line(&TextFocus::of(Ok(Some(&button))), Duration::from_secs(300)),
            "focus: AXButton → wait (up to 300s)"
        );
        assert_eq!(
            decision_line(&TextFocus::of(Ok(Some(&field))), Duration::from_secs(300)),
            "focus: AXTextField +selection +settable → paste"
        );
    }

    #[test]
    fn should_log_that_the_focused_element_lies_in_web_content() {
        let page = FocusedElement {
            in_web_content: true,
            ..element("AXWebArea")
        };
        assert_eq!(
            decision_line(&TextFocus::of(Ok(Some(&page))), Duration::from_secs(300)),
            "focus: AXWebArea +web → wait (up to 300s)"
        );
    }

    #[test]
    fn should_paste_at_once_when_a_text_input_has_focus() {
        let probe = probe("AXTextField", usize::MAX, "AXTextField");
        let mut paster = pasting_once();
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(0),
            WAIT,
            &mut || false,
        )
        .unwrap();
        assert_eq!(outcome, Insertion::Pasted);
    }

    #[test]
    fn should_paste_at_once_when_focus_cannot_be_read() {
        let mut probe = MockFocusProbe::new();
        probe
            .expect_focused_element()
            .returning(|| Err(anyhow::anyhow!("AX error")));
        let mut paster = pasting_once();
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(0),
            WAIT,
            &mut || false,
        )
        .unwrap();
        assert_eq!(outcome, Insertion::Pasted);
    }

    #[test]
    fn should_wait_then_paste_when_a_text_input_gets_focus() {
        let probe = probe("AXButton", 3, "AXTextField");
        let mut paster = pasting_once();
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(1),
            WAIT,
            &mut || false,
        )
        .unwrap();
        assert_eq!(outcome, Insertion::PastedAfterWait);
    }

    #[test]
    fn should_leave_text_in_clipboard_when_the_wait_times_out() {
        let probe = probe("AXButton", usize::MAX, "AXButton");
        let mut paster = leaving_in_clipboard();
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(1),
            WAIT,
            &mut || false,
        )
        .unwrap();
        assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::TimedOut));
    }

    #[test]
    fn should_end_the_wait_on_a_hotkey_press_and_consume_it() {
        let probe = probe("AXButton", usize::MAX, "AXButton");
        let (tx, rx) = mpsc::channel();
        tx.send(PipelineInput::Hotkey(PttSignal::Pressed)).unwrap();
        let mut pending = VecDeque::new();
        let mut paster = leaving_in_clipboard();
        let cfg = FocusWaitCfg {
            timeout: Duration::from_secs(60),
            ..WAIT
        };
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(1),
            cfg,
            &mut || hotkey_pressed_during_wait(&rx, &mut pending),
        )
        .unwrap();
        assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::HotkeyPressed));
        assert!(rx.try_recv().is_err());
        assert!(pending.is_empty());
    }

    #[test]
    fn should_end_the_wait_on_cancel() {
        let probe = probe("AXButton", usize::MAX, "AXButton");
        let (tx, rx) = mpsc::channel();
        tx.send(PipelineInput::Hotkey(PttSignal::Cancel)).unwrap();
        let mut pending = VecDeque::new();
        let mut paster = leaving_in_clipboard();
        let cfg = FocusWaitCfg {
            timeout: Duration::from_secs(60),
            ..WAIT
        };
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(1),
            cfg,
            &mut || hotkey_pressed_during_wait(&rx, &mut pending),
        )
        .unwrap();
        assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::HotkeyPressed));
    }

    #[test]
    fn should_keep_waiting_on_a_release() {
        let probe = probe("AXButton", usize::MAX, "AXButton");
        let (tx, rx) = mpsc::channel();
        tx.send(PipelineInput::Hotkey(PttSignal::Released)).unwrap();
        let mut pending = VecDeque::new();
        let mut paster = leaving_in_clipboard();
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(1),
            WAIT,
            &mut || hotkey_pressed_during_wait(&rx, &mut pending),
        )
        .unwrap();
        assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::TimedOut));
        assert_eq!(pending, [PipelineInput::Hotkey(PttSignal::Released)]);
    }

    #[test]
    fn should_keep_a_voice_input_that_arrives_during_the_wait() {
        let probe = probe("AXButton", usize::MAX, "AXButton");
        let (tx, rx) = mpsc::channel();
        tx.send(PipelineInput::SpottingLost).unwrap();
        let mut pending = VecDeque::new();
        let mut paster = leaving_in_clipboard();
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(1),
            WAIT,
            &mut || hotkey_pressed_during_wait(&rx, &mut pending),
        )
        .unwrap();
        assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::TimedOut));
        assert_eq!(pending, [PipelineInput::SpottingLost]);
    }

    #[test]
    fn should_paste_without_probing_when_the_wait_is_off() {
        let mut probe = MockFocusProbe::new();
        probe.expect_focused_element().never();
        let mut paster = pasting_once();
        let outcome = insert_text(
            &text(),
            &mut paster,
            Some(&probe),
            &indicator(0),
            FocusWaitCfg::OFF,
            &mut || false,
        )
        .unwrap();
        assert_eq!(outcome, Insertion::Pasted);
    }

    #[test]
    fn should_paste_without_probing_when_there_is_no_probe() {
        let mut paster = pasting_once();
        let outcome = insert_text(&text(), &mut paster, None, &indicator(0), WAIT, &mut || {
            false
        })
        .unwrap();
        assert_eq!(outcome, Insertion::Pasted);
    }
}
