# Proposal

## Why

The app pastes the final text with Cmd+V into whatever has keyboard focus. When no text input is focused — the user clicked the desktop, a toolbar or a list while the dictation was being processed — the keystroke goes nowhere and the dictation is lost. The user wants the app to notice that, wait until they focus a text input, and paste then.

## What Changes

- Before pasting, the app asks macOS (Accessibility API, permission already granted) what has keyboard focus.
- **Paste-first rule — the wait must never eat a paste that would have worked.** The app waits only when it is *sure* no text input is focused: the focused element has a role from a fixed list of clearly non-text controls (button, list, table, outline, image, menu, check box, radio button, slider, tab group, toolbar) **and** exposes neither a text selection range nor a settable value. Every other case pastes immediately, as today: a text role, any unknown or generic role (web area, group — what Electron apps and browsers report), no focused element reported, an Accessibility error or timeout.
- **Waiting** → the overlay shows a new "waiting for focus" state and the app re-checks focus every ~100 ms; as soon as the paste-first rule says "paste", it pastes and hides the overlay.
- **Wait ends without focus** — the configured time passes (default **5 minutes**), or the user presses the hotkey again — the text is left in the clipboard (no Cmd+V, prior clipboard not restored), the overlay hides and the log says the text is in the clipboard. A hotkey press that ends the wait does not start a new recording; the next press does.
- **Settings** in the config file, `[paste]` section: the wait time in seconds (default 300); `0` turns the wait off, so the app always pastes at once as today.
- **While the focus wait is on, every paste logs the focus decision** (role seen, paste / wait / unknown), so a wrong wait can be reported and the role list fixed.
- `--stdout` output (stdio paster) is unchanged: no focus check.

## Capabilities

### New Capabilities

- `text-insertion`: how the final text reaches the user's app — paste into the focused text input, wait for a text input to get focus, leave the text in the clipboard when the wait ends.

### Modified Capabilities

_None._ (`push-to-talk` keeps its requirement; the hotkey press that ends a wait is specified in `text-insertion`.)

## Impact

- Code: new port `FocusProbe` in `app/src/core/application/` and its macOS adapter (`AXUIElementCreateSystemWide` → `AXFocusedUIElement` → role / settable value) under `app/src/adapters/secondary/`; `Pipeline` waits for focus before paste and watches the hotkey channel while it waits; `PasteCfg` gains the wait-time setting; `TextPaster` gains a "leave in clipboard" path; `VisualIndicator` gains `show_waiting_for_focus` with a macOS overlay state.
- Tests: unit tests for the pipeline wait logic with mocked probe, paster, indicator and clock; slice `.feature` for `text-insertion`; manual run for the real focus detection and overlay.
- Docs: `docs/ubiquitous-language.md` adds *text input focus* and *focus wait*.
- Risk and trade-off: the role list is deliberately narrow, so some truly unfocused cases (an Electron app with no input focused) still paste into nothing, as today — a missed wait costs what it costs now, a wrong wait would cost a paste. An app that reports a listed non-text role while a text input really has focus delays the paste; the hotkey press or the timeout leaves the text in the clipboard, so nothing is lost, and the logged role lets the list be fixed.

## Scope

- **Lifecycle stage:** PoC, stays PoC. Acceptance bar: unit and slice tests pass in `just check`; `just full-check` passes before merge; the user's manual run shows the wait-then-paste and the timeout-to-clipboard cycles, and pastes into a terminal, a browser text field and an Electron app with no wait in the log.
- **Out of scope:** pasting into the app that was focused when recording started (focus tracking across apps); inserting text via the Accessibility API instead of Cmd+V; per-app exclusion lists; a sound or system notification when the wait ends.

## Release

MINOR — new capability; target version **0.5.0**.
