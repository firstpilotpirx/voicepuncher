## Why

Manual testing of the liquid glass overlay redesign (v0.24.0) on macOS 26 found four problems:

1. **White line on top of every glass panel.** A bright white 1 pt straight line runs along the top edge of the pill in every state (recording, Pasted, others) and of the text window under it. It looks like the border did not join up: the line is the "top highlight" of the design (`mockups/design-spec.md` of the archived `redesign-overlay-liquid-glass`: 1 pt, white 85 %), drawn as a straight segment between the corner radii, so it ends abruptly where the corners start; on macOS 26 it also doubles the edge the system glass (`NSGlassEffectView`) already draws, on top of our own 60 % white rim. The code is shared by every panel (`glass.rs`), so every panel shows it.
2. **No live transcript while recording.** While the user speaks, the text recognized so far no longer appears under the pill, though it did before the redesign. A screenshot shows only a blurred patch just under the pill's bottom edge.
3. **The text window overlaps the pill.** After Pasted, the pill covers the top part (about 20 pt) of the text window under it; the text window should hang 8 pt below the pill. Hypothesis from the code, not yet verified at runtime: the pill sits 4 pt under the top of the screen's visible frame (`GAP_BELOW_SCREEN_TOP`), but its window has a 32 pt margin around the glass for the shadow (`SHADOW_MARGIN`), so the window reaches into the menu bar and macOS pushes it down by about 28 pt, while the text window is placed from the unshifted pill rectangle (`TranscriptWindow::build`). Problem 2 is then most likely the same bug: the live transcript is shown but hidden behind the pill.
4. **The text window shows outside recording.** The text window under the pill now also shows the refinement preview, the waiting-for-focus explanation, the pasted text after Pasted and the failure message. The user wants it only while recording, for the live transcript; in every other state it should not show. This is what the current `visual-feedback` spec requires, so it is a requirement change, not only an implementation bug.
5. **The pill changes size between states.** The pill shown at the start of a recording and the pills that follow (recording with the waveform, processing stages, Pasted and others) have different widths: the pill grows or shrinks around its centre as it needs (`WindowSet::set_pill_width`, starting from `INITIAL_PILL_WIDTH`). The user wants every state of the pill to be the same size, so the overlay looks calmer and neater.
6. **The glass is almost opaque.** Over a dark window and over a bright blue desktop alike, the pill and the text window look nearly solid: what lies under them barely shows through. The user wants the glass more transparent — clearly translucent, with the picture under the panel visible through it, blurred.

## What Changes

- No white line along the top edge of any glass panel: the pill in every state and the text window show a clean rounded edge.
- The live transcript is visible under the pill while recording.
- The text window hangs below the pill and never overlaps it, on every screen.
- The text window under the pill shows only during recording (the live transcript); refinement, waiting for focus, Pasted and failure notices show no text window.
- The pill has one fixed size in every state, from the start of the recording to Pasted.
- The glass of the pill and the text window is clearly translucent: the window under it shows through, blurred.

Current stage: PoC. Target stage: PoC.
Scope: the macOS overlay's glass panels (pill and text window): edge drawing, window placement, the pill's size, the glass's transparency, and which states show the text window.
Acceptance bar: on the user's manual run on macOS 26, no white line on any panel, the live transcript appears under the pill while speaking and does not overlap it, Pasted / refinement / waiting / failure show no text window, the pill keeps one size through every state, and the blurred window under the glass shows through it; `visualfeedback_macos_it` and the `visual-feedback` scenarios pass.
Out of scope: the pointer badge (dictation sign), the waveform, colours and tints, the fallback look on macOS before 26 beyond removing the same line.

## Capabilities

### Modified Capabilities

- `visual-feedback`: the text window under the pill shows only the live transcript during recording; the refinement preview, waiting explanation, pasted text and failure message no longer show in it; the text window never overlaps the pill; the pill keeps one size in every state.

## Impact

- `app/src/adapters/secondary/visualfeedback/macos/glass.rs` (top highlight, rim), `window_set.rs` (pill placement and width), `pill_view.rs` / `pill_content.rs` (content fitted to one width), `transcript_window.rs` (anchor to the pill), `pill_states.rs` (which states show the text window), possibly `failure_notice.rs`.
- `openspec/specs/visual-feedback/spec.md` and its `.feature` scenarios.
- Manual run by the user (overlay change).

## Open points

- Cause of problems 2 and 3 is a hypothesis: confirm by logging the pill window's real frame after it is shown before fixing. Proposed fix: keep the pill window inside the visible frame (or account for the shift) and place the text window from the pill's real frame.
- Whether to keep our own rim on macOS 26 at all, or rely on the system glass edge only. Proposed: drop the top highlight everywhere, drop our rim on macOS 26, keep it on the fallback material.
- Which fixed size the pill takes: proposed the width of the widest state (recording with the waveform), with shorter states centred in it; the commands-unavailable sign alone may stay compact.
- How much transparency, and how to get it on macOS 26 (the system glass decides its own opacity; a lighter tint, a clear variant of the system glass, or a lighter fallback material). Proposed: try the system's clearer glass first, and keep the content readable over light and dark windows.
- With the text window gone outside recording, where the failure message and the waiting-for-focus explanation go: the user said the text window belongs only to recording. Proposed: the pill's own label and icon carry them; no text window.

## Release

SemVer: PATCH — bug fixes of the overlay redesign plus a narrowing of what the text window shows.
Target version: 0.24.1.
