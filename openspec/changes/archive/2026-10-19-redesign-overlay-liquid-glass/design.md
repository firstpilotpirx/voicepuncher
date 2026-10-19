# Design

## Context

The overlay is drawn by `MacosVisualIndicator` (`adapters/secondary/visualfeedback/macos/`) behind the `VisualIndicator` port; `PointerSignIndicator` decorates it and shows the 🎙️ sign only while recording. The microphone level already reaches the overlay through a shared atomic register, read by a 60 ms equalizer tick. After a paste the pipeline calls `hide()`; the port has no signal for "pasted" and `show_refining_text()` carries no text. `objc2-app-kit` 0.3.2 binds `NSGlassEffectView`. The look itself is fixed by `mockups/design-spec.md`.

## Decisions

**D1 — Two port changes, no new port.** `VisualIndicator` gains `show_pasted(text)`, and `show_refining_text` takes the raw transcript. The pipeline calls `show_pasted` after `Pasted` / `PastedAfterWait` where it called `hide()`; a pending failure notice (part of the dictation lost) still wins over it. Alternative: the adapter infers "pasted" from `hide()` after processing — rejected, `hide()` also ends cancelled, empty and clipboard dictations.

**D2 — The adapter times the Pasted sign.** It shows 0.5 s, then hides, guarded by the overlay's generation counter like the command sign, so any later signal replaces it. The core stays free of display timing.

**D3 — The pointer badge mirrors the decorator's signals.** `PointerSignView::show` takes a badge state (recording, recognizing, refining, waiting, pasted, failure); `PointerSignIndicator` maps every overlay signal to it, leaves it on a command sign or the commands-unavailable sign, and hides it with `hide()`. Pasted and failure badges hide themselves after the same time as their overlay sign (0.5 s, 4 s). The badge state enum lives in the adapter: it is presentation, not domain.

**D4 — System glass with a runtime fallback.** On macOS 26 the pill and text window host an `NSGlassEffectView` (tint for waiting and failure); elsewhere an `NSVisualEffectView` (`.hudWindow`, behind-window, active) — chosen by a class lookup at start, so one binary runs on both. Content uses system label colours, so light/dark follows the glass. Alternative hand-drawn blur — rejected: cannot sample other apps' windows.

**D5 — SF Symbols and layer animations.** Icons are `NSImage(systemSymbolName:)` with a medium symbol configuration and palette colour; breathing and the record-dot pulse are `CABasicAnimation`s on the image layer (symbol effects lack bindings). Spinners stay `NSProgressIndicator` spinning. Reduce Motion is read from `NSWorkspace` each time a state is applied.

**D6 — Waveform model in the domain, drawing in the adapter.** A pure `LevelHistory` (replacing `level_to_pills`) holds the smoothed level, the peak per 100 ms column and a ring of 30 columns, and yields each column's height and the slide offset for a given time; it is unit-tested with a fake clock. The adapter draws the columns as rounded `CALayer`s in one container with a left alpha mask, moved on the existing ~16 ms main-queue tick (not a display link: same smoothness, no new binding).

**D7 — English labels in pure mapping functions.** State → (symbol, label, tint) lives in plain functions of the adapter, unit-tested, replacing the glyph constants.

## Risks / Trade-offs

- [`NSGlassEffectView` looks differently from the mockups' imitation] → the manual run compares with `mockups/states/`; tune tint alpha only.
- [A 16 ms tick on the main queue costs CPU during long recordings] → it runs only while recording and stops on the generation change, as the equalizer does now.
- [The Pasted sign flashes between "stop" and a following command sign] → the command sign replaces it at once (D2).

## Test levels

| Scenario | Level |
|---|---|
| A hotkey recording shows the dictation sign | component `dictation_pointer.feature` |
| A recording started by voice shows the dictation sign | component `dictation_pointer.feature` |
| The dictation sign sits close to the pointer's tip | adapter integration (`visualfeedback_macos_it`) |
| The dictation sign mirrors every stage of a dictation | component `dictation_pointer.feature`; icons in manual run |
| A failure shows a warning triangle in the dictation sign | component `dictation_pointer.feature` |
| A command sign leaves the dictation sign as it was | component `dictation_pointer.feature` |
| A pasted dictation removes the dictation sign with the overlay | component `dictation_pointer.feature` |
| Cancelling a recording removes the dictation sign | component `dictation_pointer.feature` |
| A recording too short to keep removes the dictation sign | component `dictation_pointer.feature` |
| Restarting a dictation shows the sign for the new recording | component `dictation_pointer.feature` |
| Quitting the app during a recording leaves the normal pointer | manual run |
| Commands unavailable in standby show no dictation sign | component `dictation_pointer.feature` |
| Text refinement follows speech recognition | component `processing_stages.feature` |
| With text refinement disabled only speech recognition is shown | component `processing_stages.feature` |
| Text held back for focus shows the wait after text refinement | component `processing_stages.feature` |
| Speech recognition shows an ear left of its label and spinner | unit (state mapping); manual run |
| Text refinement shows sparkles left of its label and spinner | unit (state mapping); manual run |
| The two stages look different | unit (state mapping) |
| The stage icons read in light and dark appearance | manual run |
| A long speech recognition never looks nearly done | manual run |
| Refinement shows the dimmed raw transcript | component `processing_stages.feature`; dimming in manual run |
| Paste once the user clicks into a text field | slice `text_insertion.feature` |
| Paste once the user clicks into a text input on the web page | slice `text_insertion.feature` |
| The overlay over a dark window shows light content | manual run |
| The overlay over a light window shows dark content | manual run |
| The glass overlay takes no focus | adapter integration (`visualfeedback_macos_it`) |
| The processing stages read in English | unit (state mapping) |
| The recording shows no label | unit (state mapping) |
| Speech leaves tall columns that slide to the left | unit (`LevelHistory`); motion in manual run |
| Silence leaves a dotted line | unit (`LevelHistory`) |
| A new recording starts with a dotted waveform | unit (`LevelHistory`) |
| The waveform keeps the last three seconds | unit (`LevelHistory`) |
| The record dot pulses in silence too | manual run |
| Held-back text shows the orange wait sign | unit (state mapping); manual run |
| A pasted dictation shows the Pasted sign | component `visual_feedback.feature`; 0.5 s hide in adapter integration |
| Text pasted after a wait for focus shows the Pasted sign | component `visual_feedback.feature` |
| Text left in the clipboard shows no Pasted sign | component `visual_feedback.feature` |
| Text pasted without refinement shows the failure notice instead | unit (`pipeline.rs`, pending notice wins over the Pasted sign) |
| A command right after the paste replaces the Pasted sign | component `visual_feedback.feature` |
| The enter sign shows its icon and name | unit (state mapping); manual run |
| A lost dictation shows the amber notice with its message | unit (state mapping); manual run |
| A failure notice over a white window stays legible | manual run |
| Standby shows the small CMD pill | unit (state mapping); manual run |
| A recording shows the microphone-off icon after the waveform | manual run |
| Reduce Motion keeps the record dot still | manual run |
| A hotkey dictation shows no command sign | component `voice_control.feature` |
