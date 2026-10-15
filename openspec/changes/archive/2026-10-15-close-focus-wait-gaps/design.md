# Design

## Context

The focus wait (`text-insertion`, archived change `wait-for-text-focus`) decides in `TextFocus::of` (`app/src/core/domain/text_focus.rs`) from a `FocusedElement { role, has_text_selection, value_settable }` that `MacOsFocusProbe` (`app/src/adapters/secondary/focus/macos/`) reads through the Accessibility API. It waits only on a fixed list of non-text roles. Browsers and Electron apps report their page as `AXWebArea` or a group (`AXGroup`) when no input on it is focused, so the text is pasted with no input focused. Chrome and Electron build their web accessibility tree only when an assistive client asks: without it, a focused `<input>` may not be reported as `AXTextField` at all, which is why the first design refused to wait on generic roles.

Generic roles are also what terminals and custom-drawn apps (Alacritty, kitty, some Electron-free editors) report for their real text input, so waiting on every generic role would block their pastes.

## Goals / Non-Goals

**Goals:**
- Hold the text back in web content when no text input on the page is focused.
- A focused text input on a page, a contenteditable element and every input outside web content keep pasting at once.
- The rule stays pure core code, unit-tested on every branch.

**Non-Goals:**
- Waiting in generic elements outside web content.
- `AXEnhancedUserInterface` (it disturbs window managers and window animations).
- Per-app lists, AX text insertion.

## Decisions

### D1. The probe reports whether the element lies in web content
`FocusedElement` gains `in_web_content: bool`. `MacOsFocusProbe` sets it when the focused element's role is `AXWebArea` or one of its ancestors (`AXParent`, at most 64 steps, stopping at the window or application) is `AXWebArea`. An AX error while walking up ends the walk with `false`: an unknown position means "not in web content", so the old rule applies and the text is pasted.
*Alternative:* bundle-id list of browsers — rejected: misses Electron apps and web views in other apps, and needs upkeep.

### D2. The rule for web content
`TextFocus::of` waits when either
- the role is in the non-text list and both flags are false (unchanged), or
- `in_web_content` is true, the role is not a text role (`AXTextField`, `AXTextArea`, `AXComboBox`, `AXSearchField`) and the value is not settable.

A text selection is ignored inside web content: a page reports the user's page text selection, which is no input. Chrome reports a contenteditable root as `AXTextArea` with a settable value, so rich editors (Slack, Gmail, Notion) paste at once. Everything else pastes as before.

### D3. Ask the app to expose its web content
Before reading the focused element, `MacOsFocusProbe` reads `AXFocusedApplication` from the system-wide element and sets `AXManualAccessibility = true` on it. Electron and Chromium build their web accessibility tree when this attribute is set; other apps reject it, and the error is ignored. Setting it is idempotent, so it runs on every probe (one extra AX call, ≤ 0.2 s messaging timeout) and covers apps started after voicepuncher.
When the app did not expose its page yet, the focused element is a window-level group outside any `AXWebArea`, so D2 pastes at once — the old behavior. If the tree is still being built when the decision is taken, the wait re-probes every 100 ms and pastes as soon as the input is reported.

### D4. The log line
`FocusedElement`'s description appends ` +web` when `in_web_content` is set, e.g. `focus: AXWebArea +web → wait (up to 300s)`, so the log shows why the app waited.

### D5. Glossary
`docs/ubiquitous-language.md` § Text input focus gets the web content rule; no new term beyond "web content".

## Test levels

| Spec scenario | Level |
|---|---|
| Paste at once into a focused text field | slice `.feature` (+ unit) |
| Paste at once when the app does not tell what has focus | slice `.feature` (+ unit) |
| Paste at once into a text input on a web page | slice `.feature` (+ unit for each text role in web content) |
| Paste at once into an editable element on a web page | slice `.feature` (+ unit) |
| Hold the text back when a web page has focus without a text input | slice `.feature` (+ unit for `AXWebArea`, `AXGroup` in web content) — reproduces the bug |
| Hold the text back when text is selected on a web page | slice `.feature` (+ unit) |
| Paste at once into an element of a generic kind | slice `.feature` (+ unit for `AXWindow`, `AXGroup`, `AXScrollArea`, `AXUnknown`) |
| Paste at once into a list that offers a text selection | slice `.feature` (+ unit) |
| Hold the text back when a button has focus | slice `.feature` |
| Paste once the user clicks into a text field | slice `.feature`; real AX: manual run |
| Paste once the user clicks into a text input on the web page | slice `.feature`; real AX in Chrome and an Electron app: manual run |
| The log names the focused element and the decision | unit (`text_insertion` decision line) |
| The log says the focused element lies in web content | unit (`TextFocus` description + decision line) |

The real AX probe (D1, D3) gets the gated adapter integration test `app/tests/focus_macos_it.rs` (`VOICEPUNCHER_REAL_AX_IT`); real browsers and Electron apps are checked in the manual run.

## Risks / Trade-offs

- [Chrome ignores `AXManualAccessibility`] → its page stays invisible, the focus is a window-level group outside web content, the text is pasted at once as today; the log (no `+web`) shows it, and `AXEnhancedUserInterface` can follow in a later change.
- [A web app draws its own input without a text role or settable value (canvas editors)] → the text waits; clicking elsewhere, the hotkey or the timeout leaves it in the clipboard; the log names the role.
- [Walking `AXParent` costs AX calls] → at most 64 local calls per probe, only while the focus wait is on.
- [Exposing web accessibility makes Electron apps a bit slower] → the same cost any screen reader or window manager already causes; only apps the user dictates into get it.

## Migration Plan

No config change. Rollback: `focus_wait_seconds = 0`, or the previous release.
