use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use super::*;
use crate::core::application::focus_probe::MockFocusProbe;
use crate::core::application::text_paster::MockTextPaster;
use crate::core::application::visual_indicator::MockVisualIndicator;
use crate::core::domain::FocusedElement;

const WAIT: FocusWaitCfg =
    FocusWaitCfg { timeout: Duration::from_millis(30), poll_interval: Duration::from_millis(1) };

fn element(role: &str) -> FocusedElement {
    FocusedElement {
        role: role.to_owned(),
        has_text_selection: role == "AXTextField",
        value_settable: role == "AXTextField",
        in_web_content: false,
    }
}

fn text() -> CleanedText {
    CleanedText::new("hello").unwrap()
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
    let page = FocusedElement { in_web_content: true, ..element("AXWebArea") };
    assert_eq!(
        decision_line(&TextFocus::of(Ok(Some(&page))), Duration::from_secs(300)),
        "focus: AXWebArea +web → wait (up to 300s)"
    );
}

#[test]
fn should_paste_at_once_when_a_text_input_has_focus() {
    let probe = probe("AXTextField", usize::MAX, "AXTextField");
    assert_eq!(
        insertion(Some(&probe), pasting_once(), WAIT, 0),
        Insertion::Pasted
    );
}

#[test]
fn should_paste_at_once_when_focus_cannot_be_read() {
    let mut probe = MockFocusProbe::new();
    probe
        .expect_focused_element()
        .returning(|| Err(anyhow::anyhow!("AX error")));
    assert_eq!(
        insertion(Some(&probe), pasting_once(), WAIT, 0),
        Insertion::Pasted
    );
}

#[test]
fn should_wait_then_paste_when_a_text_input_gets_focus() {
    let probe = probe("AXButton", 3, "AXTextField");
    assert_eq!(
        insertion(Some(&probe), pasting_once(), WAIT, 1),
        Insertion::PastedAfterWait
    );
}

#[test]
fn should_leave_text_in_clipboard_when_the_wait_times_out() {
    let probe = probe("AXButton", usize::MAX, "AXButton");
    assert_eq!(
        insertion(Some(&probe), leaving_in_clipboard(), WAIT, 1),
        Insertion::LeftInClipboard(WaitEnd::TimedOut)
    );
}

#[test]
fn should_end_the_wait_on_a_hotkey_press_and_consume_it() {
    let cfg = FocusWaitCfg { timeout: Duration::from_secs(60), ..WAIT };
    let (outcome, pending, rx) = waited_through(PipelineInput::Hotkey(PttSignal::Pressed), cfg);
    assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::HotkeyPressed));
    assert!(rx.try_recv().is_err());
    assert!(pending.is_empty());
}

#[test]
fn should_end_the_wait_on_cancel() {
    let cfg = FocusWaitCfg { timeout: Duration::from_secs(60), ..WAIT };
    let (outcome, _, _) = waited_through(PipelineInput::Hotkey(PttSignal::Cancel), cfg);
    assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::HotkeyPressed));
}

#[test]
fn should_keep_waiting_on_a_release() {
    let (outcome, pending, _) = waited_through(PipelineInput::Hotkey(PttSignal::Released), WAIT);
    assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::TimedOut));
    assert_eq!(pending, [PipelineInput::Hotkey(PttSignal::Released)]);
}

#[test]
fn should_keep_a_voice_input_that_arrives_during_the_wait() {
    let (outcome, pending, _) = waited_through(PipelineInput::SpottingLost, WAIT);
    assert_eq!(outcome, Insertion::LeftInClipboard(WaitEnd::TimedOut));
    assert_eq!(pending, [PipelineInput::SpottingLost]);
}

#[test]
fn should_paste_without_probing_when_the_wait_is_off() {
    let mut probe = MockFocusProbe::new();
    probe.expect_focused_element().never();
    assert_eq!(
        insertion(Some(&probe), pasting_once(), FocusWaitCfg::OFF, 0),
        Insertion::Pasted
    );
}

#[test]
fn should_paste_without_probing_when_there_is_no_probe() {
    assert_eq!(insertion(None, pasting_once(), WAIT, 0), Insertion::Pasted);
}

/// What inserting the text with `paster` does while `probe` answers, waiting as `cfg` says and
/// showing the wait `waits` times, with no hotkey pressed.
fn insertion(
    probe: Option<&MockFocusProbe>,
    mut paster: MockTextPaster,
    cfg: FocusWaitCfg,
    waits: usize,
) -> Insertion {
    let probe = probe.map(|probe| probe as &dyn FocusProbe);
    insert_text(
        &text(),
        &mut paster,
        FocusWait { probe, cfg },
        &indicator(waits),
        &mut || false,
    )
    .unwrap()
}

/// A wait for focus that never comes, with `input` arriving during it: the insertion, the
/// inputs kept for later and the channel the input came on.
fn waited_through(
    input: PipelineInput,
    cfg: FocusWaitCfg,
) -> (
    Insertion,
    VecDeque<PipelineInput>,
    mpsc::Receiver<PipelineInput>,
) {
    let probe = probe("AXButton", usize::MAX, "AXButton");
    let (tx, rx) = mpsc::channel();
    tx.send(input).unwrap();
    let mut pending = VecDeque::new();
    let mut paster = leaving_in_clipboard();
    let outcome = insert_text(
        &text(),
        &mut paster,
        FocusWait { probe: Some(&probe), cfg },
        &indicator(1),
        &mut || hotkey_pressed_during_wait(&rx, &mut pending),
    )
    .unwrap();
    (outcome, pending, rx)
}
