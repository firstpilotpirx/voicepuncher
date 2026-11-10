## Why

The keys VoicePuncher presses in the focused app — Cmd+V of every paste (dictation and the "paste text" command), Return of "press enter", Cmd+A and Delete of "clear all text" — are posted as a key-down and a key-up back to back, with no pause and no Command key of their own: the V event only carries the Command flag. Keyboard tools that watch the keys miss such a press: the user's BetterTouchTool trigger on Cmd+V in Ghostty never fires on a VoicePuncher paste. A test on 2026-11-10 posting the same events from a small program pasted into Ghostty with a 10 ms pause after each event and pasted nothing without the pause.

## What Changes

- Every key VoicePuncher presses in the focused app is pressed as a keyboard does: each modifier as a key of its own (Command down, V down, V up, Command up), with a pause of 10 ms after every key event.
- The paste of a dictation and the voice commands' key presses share one way to press keys.
- The events are posted at the HID level, where keyboard tools such as BetterTouchTool listen (the manual run showed they miss session-level events).

Current stage: PoC. Target stage: PoC.
Scope: the macOS key presser and paster adapters (`app/src/adapters/secondary/keypressing/macos/`, `app/src/adapters/secondary/textpasting/macos/`), a text-insertion requirement and its slice scenarios.
Acceptance bar: the slice scenarios show the order and the pauses of the events; `just check` passes; in the user's manual run the BetterTouchTool trigger on Cmd+V fires on a voice "paste text" in Ghostty.
Out of scope: the hotkey listener, the clipboard handling.
Interface: none.

## Capabilities

### Modified Capabilities

- `text-insertion`: keys pressed in the focused app arrive as a keyboard's, modifier keys of their own and a pause between events.

## Impact

- `app/src/adapters/secondary/keypressing/macos/`, `app/src/adapters/secondary/textpasting/macos/mod.rs`, `app/tests/features/slice/text_insertion.feature`, `app/tests/steps/text_insertion/`.

## Release

SemVer: PATCH — a bug fix.
Target version: 0.48.1.
