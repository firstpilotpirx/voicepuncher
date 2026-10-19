# Design

## Context

macOS lets an app set the pointer image only over its own windows, so the app cannot change the pointer over other apps. The overlay (`visualfeedback/macos`) already shows borderless, click-through windows above every app on every Space and drives them from the main queue with `dispatch_after` timers. The pipeline already tells the overlay when a recording starts (`show_recording`) and when it ends (every other `VisualIndicator` call), for the hotkey and for voice control alike.

## Goals / Non-Goals

**Goals:** a dictation sign next to the pointer during every recording, gone at its end, on every screen, never taking input.

**Non-Goals:** changing the system pointer image; a sign while processing; a sign that reacts to the voice level.

## Decisions

### A small window that follows the pointer, not a pointer image

The dictation sign is one borderless, transparent window (about 28 × 28 pt) holding a 🎙️ label, like the overlay's stage icons. It ignores mouse events, never becomes key or main, joins all Spaces and full-screen apps, and sits at the pop-up menu window level so it stays above app windows and the overlay. While shown, a main-queue timer every 16 ms reads the global pointer location and moves the window so its top-left corner sits 16 pt right of and 16 pt below the pointer's hot spot; it moves only when the location changed. Global window coordinates span every screen, so following across screens needs no screen logic. Rejected: `NSCursor` (works only over the app's own windows) and private CoreGraphics cursor APIs (unsupported, may break per macOS release).

### The sign is driven by the overlay's signals, in the adapter layer

A decorator `PointerSignIndicator` implements `VisualIndicator`: it forwards every call to the overlay, shows the dictation sign on `show_recording` and hides it on the processing stages, the wait for focus and `hide`; signals that do not end a recording (a command sign, a failure notice, the live transcript view) pass through and leave the sign as it is. The core does not change: it already sends these signals for the hotkey, voice control, cancel, restart, too-short drops and failures. The sign's view sits behind an adapter-internal trait (`PointerSignView`: show, hide) so the decorator is tested with a fake and `main.rs` wires the macOS view. Rejected: a new core port with its own calls — it would repeat every overlay call in the pipeline and could drift from it.

### Quitting needs no code

The sign window belongs to the app's process; when the process exits, the window server removes it, and the pointer was never changed.

### Placement math is a pure function

The sign's origin from the pointer location and the sign's size is a pure function, unit-tested; Cocoa's bottom-left origin makes "below" a smaller y.

## Test levels

| Scenario | Level |
|---|---|
| A hotkey recording shows the dictation sign | component `.feature` (real pipeline, fake sign view) |
| A recording started by voice shows the dictation sign | component `.feature` |
| Processing shows no dictation sign | component `.feature` |
| Stopping a recording removes the dictation sign | component `.feature` |
| Cancelling a recording removes the dictation sign | component `.feature` |
| A recording too short to keep removes the dictation sign | component `.feature` |
| Restarting a dictation shows the sign for the new recording | component `.feature` |
| Quitting the app during a recording leaves the normal pointer | manual run |
| The dictation sign follows the pointer to another screen | unit (placement math) + manual run |
| Clicks pass through the dictation sign | adapter integration (window ignores mouse events, never key) + manual run |

## Risks / Trade-offs

- The sign lags the pointer by up to one 16 ms tick; acceptable for a status sign.
- macOS hides the pointer while the user types; the sign stays visible then. Accepted for the PoC.
- A 60 Hz timer runs only while recording; its cost is a pointer read and an occasional window move.

## Dependencies

This change waits for `confirm-voice-commands`, `notify-service-failures` and `show-live-transcript`: each adds a method to `VisualIndicator`, which the decorator implements and must forward, and the command sign arrives during a recording, so the decorator must know it does not end one.
