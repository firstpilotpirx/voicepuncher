## Why

The recording pill (record sign + waveform) is narrower than the Claude desktop chat input box it sits above: about 727 pt against about 922 pt on the user's screen. The user wants the pill to be the same length as that input box, so the two line up.

![Claude desktop app: the recording pill above the chat, narrower than the chat input box](assets/a33c58ba3949.webp)
![The Claude chat input box, about 922 pt wide](assets/593aa6852c21.png)
![The recording pill today, about 727 pt wide](assets/eb04bdd65147.png)

## What Changes

- The recording pill becomes about 922 pt wide, the width of the chat input box in the screenshots, by lengthening the waveform: 210 columns instead of 160, and 196 instead of 148 beside the commands-unavailable sign, so the waveform shows the last 23.1 s of the recording instead of 17.6 s.
- Every pill state that shares the one pill width (recording, transcribing, command, …) gets the new width; the lone commands-unavailable sign keeps its own small width.
- The width stays fixed: it does not follow the size of the Claude window or any other window.

Current stage: PoC. Target stage: PoC.
Scope: the pill width and the waveform column counts.
Acceptance bar: the pill measures about 922 pt in every state that uses the one pill width, the width tests pass with the new numbers, and the user confirms in a manual run that it matches the input box.
Out of scope: pill height, colors, waveform pitch and column shape, the transcript window, following another window's width.

## Capabilities

### Modified Capabilities

- `visual-feedback`: the recording pill width and the number of waveform columns.

## Impact

- `app/src/core/domain/level_history.rs` (`FULL_CAPACITY`, `CAPACITY_BESIDE_SIGN`)
- `app/src/adapters/secondary/visualfeedback/macos/pill_content/mod.rs` (`one_pill_width`) and its tests
- `app/src/adapters/secondary/visualfeedback/macos/waveform.rs` width tests
- `app/src/core/domain/level_history.rs` tests of how long speech stays visible
- `app/tests/features/component/visual_feedback.feature` (the pill width scenario)

## Decisions

- Column counts: 210 and 196 (chosen with the user at explore). 209 and 197 would let the row beside the commands-unavailable sign grow to 923 pt; 210 and 196 give exactly 922 pt.
- The capability is `visual-feedback`, which holds the pill requirements.
- The pill still fits a small screen (a 1280 pt wide display) as is.

## Release

SemVer: PATCH — a visual adjustment of an existing capability.
Target version: 0.41.4.
