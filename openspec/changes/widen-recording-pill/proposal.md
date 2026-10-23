## Why

The recording pill (the floating widget shown while recording: red record button and the level bars) is too short and its fully round ends look foreign on macOS. The user wants it as long as ChatGPT's voice input bar and shaped like macOS panels and notifications: a superellipse (continuous corner curve, "squircle"), not a capsule with half-circle ends.

The pill as it looks now:

![The recording pill now: short capsule](assets/37d79f60d948.png)

Its size and the ChatGPT voice bar it should match in length (top: our pill; bottom: ChatGPT's bar):

![Our pill above, ChatGPT's voice bar below](assets/8343fbc74f39.webp)

![ChatGPT voice bar: the target length](assets/eec510659ecb.png)

![ChatGPT voice bar, second view](assets/1be33aa050ac.png)

The corner shape to take as the model — a macOS notification's continuous corners:

![macOS notification: superellipse corners](assets/6b0b32c2460e.png)

## What Changes

- The recording pill is as long as ChatGPT's voice input bar (its bar row in the screenshots), about twice or more its current width; the level bars fill the new width.
- Its outline becomes a superellipse with continuous corners, like macOS notifications, instead of a capsule with half-circle ends.
- Its height, colors and position stay as they are.

Current stage: PoC. Target stage: PoC.
Scope: the width of the recording pill, how its level bars fill it, and its outline shape.
Acceptance bar: the pill on screen matches the ChatGPT bar's length, its corners match the macOS notification's curve, the bars fill it, and the user confirms in the manual run.
Out of scope: the pill's colors, height and position, and the other changes of `improve-recording-pill-look`.

## Capabilities

### Modified Capabilities

- `visual-feedback`: the recording pill gets the ChatGPT bar's length and a superellipse outline.

## Impact

- `app/src/adapters/secondary/visualfeedback/**` (overlay: manual run needed).

## Open points

- Exact width in points: proposed measured from the ChatGPT bar screenshot at the same screen scale.
- Corner shape: proposed the system continuous corner curve (`kCACornerCurveContinuous`) with a radius matching the notification's look, rather than a hand-drawn superellipse.
- More bars or wider spacing: proposed more bars at the same spacing.
- Overlap with `improve-recording-pill-look` (same widget): proposed to run after it, or merge into it if it has not started.

## Release

SemVer: PATCH — a visual tweak of an existing widget.
Target version: 0.37.5.
