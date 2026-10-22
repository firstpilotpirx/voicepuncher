## Why

On the first GUI launch of the bundle (v0.37.3, macOS 27.0.1) the user found the recording pill hard to read and plain. Split from `fix-first-gui-launch-bugs` (bugs 7–8).

1. **The level waveform barely shows the voice.** Quiet sounds do not show at all, and loud speech gives bars of nearly the same height, low for how loudly the user speaks; there are no sharp steps. ChatGPT's dictation waveform shows a flat dotted line in silence and clear spikes on speech, which is the wanted look. The user asks to find out how the meter maps loudness — whether it is linear or logarithmic, and whether it suppresses noise. The model is `app/src/core/domain/level_history.rs`: loudness mapped linearly from -50 to -10 dBFS, smoothed with `RISE_RATE` 0.35 and `FALL_RATE` 0.12, one column per 200 ms (its window peak), 2–28 pt high.
   ![Our pill beside ChatGPT's dictation waveform, whose bars spike on speech and flatten to dots in silence](assets/99d3ee4caef5.webp)
2. **The pill's border looks plain.** The user finds the border of the recording pill unattractive and wants a better one.
   ![The recording pill: red record button and the level waveform](assets/4370a4935020.png)

## What Changes

- The level waveform shows silence as flat dots and speech as clearly varying spikes: quiet sounds visible, loud ones not all the same height.
- The recording pill gets a better-looking border.

Current stage: PoC. Target stage: PoC.
Scope: the level waveform's mapping of loudness to bar height and its smoothing, and the pill's border.
Acceptance bar: unit tests pin the new mapping (silence → dots, speech levels of the user's voice → distinct heights); the user approves the pill's border and waveform against the ChatGPT screenshot in a mockup and in the running app.
Out of scope: other overlay elements than the border and the waveform; the permissions gate and the hotkey crash (changes `fix-permissions-and-login-item`, `fix-hotkey-crash-and-limit-warning`).

## Capabilities

### Modified Capabilities

- `visual-feedback`: the pill's border and the level waveform's mapping of loudness to bar height.

## Impact

- `app/src/core/domain/level_history.rs`, `audio_level.rs`
- the overlay adapter that draws the pill

## Open points

- The target mapping; proposed: measure the levels of the user's voice and of silence in their room, then a mapping over a wider dBFS range with a noise floor and less smoothing (per-column peak, faster rise), compared side by side with the ChatGPT screenshot.
- What "a better border" means; proposed: a mockup of two or three variants for the user to pick before code.

## Release

SemVer: PATCH — a visual fix, no new capability.
Target version: 0.37.4 (the next PATCH above `main` when it merges).
