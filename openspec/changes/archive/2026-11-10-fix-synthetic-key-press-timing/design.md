## Context

`MacOsKeyPresser::press` and `MacOsPaster::post_cmd_v` each post a key-down and a key-up `CGEvent` back to back at the session level, the modifier only as a flag on the key's events. A test program posting the same events (2026-11-10) pasted into Ghostty with a 10 ms pause after each event and nothing without it.

## Decisions

### D1: one keystroke function for both adapters

A new module `keypressing/macos/keystroke.rs` turns a key and its modifier flags into the event sequence — each modifier key down (Command for `CGEventFlagCommand`), the key down, the key up, the modifier keys up in reverse — and posts it with a pause of 10 ms (`EVENT_GAP`) after every event. The poster is a parameter, so the slice scenarios record the events and their times instead of posting them; the adapters pass the real `CGEvent` poster. The paster calls the same function for its Cmd+V. The modifier's own events carry the flags of the modifiers held at that moment (Command down carries Command, Command up carries none), as a keyboard's do. The voice commands' keys are posted at the HID level and a dictation's Cmd+V at the session level: the manual run showed BetterTouchTool hears a Cmd+V posted at the HID level and none at the session level, while the focused app takes both; the user wants their Cmd+V trigger on "paste text" and not on a dictation's paste, which would paste the clipboard a second time. Only Command is pressed as a key of its own: no press needs another modifier, and left Option or left Control can be the hotkey, whose listen-only tap at the HID level hears these presses too. Rejected: a pause only between down and up (the test that worked paused after every event); the session level (BetterTouchTool misses it).

### D2: the pause costs 40 ms per paste

Cmd+V now takes four events, 40 ms with the pauses, against 0 before. The paste stage of the log line (`paste=…ms`, about 22 ms today) grows by that much; no budget bounds it.

## Test levels

| Scenario | Level |
|---|---|
| Cmd+V presses Command as a key of its own | slice `.feature` (text_insertion), recording poster |
| Return goes down and up with a pause between | slice `.feature` (text_insertion), recording poster |
| BetterTouchTool trigger fires on a voice paste in Ghostty | manual run |
| BetterTouchTool trigger stays quiet on a dictation's paste | manual run |
