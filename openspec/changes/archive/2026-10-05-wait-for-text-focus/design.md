# Design

## Context

`Pipeline` (single thread, `app/src/core/application/pipeline.rs`) calls `TextPaster::paste` synchronously at the end of both dictation paths (`run_transcribe_refine_paste`, `run_refine_paste`). `MacOsTextPaster` puts the text in the clipboard, posts Cmd+V and restores the prior clipboard 120 ms later. Nothing looks at focus. The Accessibility permission the paste already needs also allows reading the focused element through the AX API; the project calls `ApplicationServices` through raw FFI (`adapters/secondary/permissions/macos`), so no new crate is needed.

## Goals / Non-Goals

**Goals:**
- The wait never blocks a paste that works today: only a positively identified non-text control triggers it.
- The paste-first rule is pure core code, unit-tested on every branch.
- The wait loop is testable without a real clock, AX or keyboard.

**Non-Goals:**
- File mode (`run_once`, `--file`) and `--stdout` output: they keep pasting/printing at once.
- Focus tracking across apps, AX text insertion, per-app lists.

## Decisions

### D1. The port reports facts, the core decides
New port `FocusProbe::focused_element(&self) -> Result<Option<FocusedElement>>` in `core/application/focus_probe.rs`. Domain value `FocusedElement { role: String, has_text_selection: bool, value_settable: bool }` and `TextFocus` verdict in `core/domain/text_focus.rs`: `TextFocus::of(Result<Option<&FocusedElement>>)` → `Paste { seen }` or `Wait { seen }`, where `seen` is the log description. `Wait` only when the role is in the fixed non-text list (`AXButton`, `AXList`, `AXTable`, `AXOutline`, `AXImage`, `AXMenu`, `AXMenuBar`, `AXMenuItem`, `AXCheckBox`, `AXRadioButton`, `AXSlider`, `AXTabGroup`, `AXToolbar`) **and** both flags are false. `Err` (AX error) and `Ok(None)` (no focused element) → `Paste`.
*Alternative:* allow-list of text roles and wait on everything else — rejected: Electron/Chrome report `AXGroup`/`AXWebArea` or nothing, so a real focused input would wait. A missed wait costs today's behavior; a wrong wait costs a paste.

### D2. macOS adapter via raw AX FFI
`adapters/secondary/focus/macos/` (`MacOsFocusProbe`): `AXUIElementCreateSystemWide` → `AXUIElementCopyAttributeValue(kAXFocusedUIElementAttribute)` → `AXRole`; `has_text_selection` = `AXSelectedTextRange` attribute present; `value_settable` = `AXUIElementIsAttributeSettable(AXValue)`. A messaging timeout (`AXUIElementSetMessagingTimeout`, 0.2 s) on the system-wide element keeps a hung app from stalling the paste; the timeout error maps to `Err` → paste.

### D3. Insertion is a core function, not more pipeline state
`core/application/text_insertion.rs`: `insert_text(cleaned, paster, probe, indicator, cfg: FocusWaitCfg, signals: Option<&Receiver<PttSignal>>) -> Insertion` with `Insertion::{Pasted, PastedAfterWait, LeftInClipboard(EndReason::{TimedOut, HotkeyPressed})}`. `FocusWaitCfg { timeout: Duration, poll_interval: Duration }` and the probe are set with `Pipeline::with_focus_wait(probe, cfg)`, so the existing `PipelineConfig` constructions stay as they are. `Pipeline` holds `focus_probe: Option<Box<dyn FocusProbe>>`; `None` or `timeout == 0` → paste at once with no probe call. Both dictation paths call `insert_text` instead of `paster.paste`.
Loop: probe → `Paste` → `paster.paste`; `Wait` → `indicator.show_waiting_for_focus()` once, then every `poll_interval`: `signals.try_recv()` (a `Pressed` or `Cancel` ends the wait; `Released` is dropped), probe again, check the deadline (`Instant`, so unit tests use a 30 ms timeout and 1 ms poll — no clock port needed).

### D4. The hotkey channel moves into the pipeline
`Pipeline::run(rx)` stores `rx` in `self.signals` and receives from it, so `insert_text` can see a press during the wait. The press is consumed by the wait and never reaches `step`, so it does not start a recording; the next press does. A `Released` that follows in hold mode is consumed by the wait or ignored in `Idle` as today. Tests drive `Pipeline::run` on its own thread and send signals through the channel.

### D5. "Leave in clipboard" is a paster operation
`TextPaster::leave_in_clipboard(&mut self, text)`: macOS sets the clipboard, no Cmd+V, no restore; stdio prints the text (never reached in practice, kept total).

### D6. Overlay state
`VisualIndicator::show_waiting_for_focus()`; macOS overlay: new `OverlayState::WaitingForFocus`, label `⌨ FOCUS` in orange, pulsing like REC, no spinner/equalizer; re-applied on screen rebuild like the other states.

### D7. Setting
`[paste] focus_wait_seconds` (u64, default 300) in `PasteCfg`; main builds `FocusWaitCfg { timeout: secs, poll_interval: 100 ms }`. The probe is wired only in dictation mode with the macOS paster.

### D8. Logging
While the focus wait is on (a probe and a non-zero wait time), every insertion logs `focus: <seen> → paste` or `focus: <seen> → wait (up to Ns)`; `<seen>` is the AX role with flags, `no focused element` or `unknown (<error>)`. End of a wait logs `focus: <role> after <ms>ms → paste` or `focus wait ended (<reason>) after <ms>ms: text left in clipboard`.

## Test levels

| Spec scenario | Level |
|---|---|
| Paste at once into a focused text field | slice `.feature` (+ unit for the rule) |
| Paste at once when the app does not tell what has focus | slice `.feature` (+ unit for `Err` and `None`) |
| Paste at once into an element of a generic kind | slice `.feature` (+ unit for `AXWebArea`, `AXGroup`) |
| Paste at once into a list that offers a text selection | slice `.feature` (+ unit) |
| Hold the text back when a button has focus | slice `.feature` |
| Paste once the user clicks into a text field | slice `.feature`; real AX: manual run |
| The wait time passes without a text input | slice `.feature` |
| A hotkey press ends the wait | slice `.feature` |
| The next hotkey press after an ended wait records | slice `.feature` |
| Default wait time is five minutes | unit (config) |
| A wait time of zero pastes at once | slice `.feature` |
| The log names the focused element and the decision | unit (`TextFocus` description + log line builder) |

Slice tests drive the real `Pipeline` with stand-ins: recorder, transcriber (fixed text), passthrough refiner, a scripted `FocusProbe` (a sequence of answers; it can push a `Pressed` into the channel to simulate the user), a recording paster and indicator. The real AX adapter gets a plain-Rust integration test `app/tests/focus_macos_it.rs` gated by `VOICEPUNCHER_REAL_AX_IT` (needs Accessibility for the test binary). The overlay and real focus detection: manual run.

## Risks / Trade-offs

- [An app reports a listed role while its text input really has focus] → paste delayed; the hotkey press or the timeout leaves the text in the clipboard; the logged role lets the list be fixed.
- [AX call hangs on a busy app] → 0.2 s messaging timeout, error → paste.
- [The pipeline thread is busy during a 5-minute wait] → a hotkey press ends it at once, so the user is never locked out.
- [Probe polling cost] → one AX round trip per 100 ms only while waiting.

## Migration Plan

New config key with a default; existing config files keep working (`deny_unknown_fields` only rejects unknown keys). Rollback: `focus_wait_seconds = 0`.
