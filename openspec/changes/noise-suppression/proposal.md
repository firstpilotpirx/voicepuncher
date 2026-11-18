## Why

With background noise dictation gets much worse. Noise suppression before recognition (for example DeepFilterNet) might help, but it can also make recognition worse: suppressors distort speech, and Whisper is trained on noisy audio. It needs thinking and measuring before it is turned on.

## What Changes

- Think it through first: whether to suppress noise at all, which suppressor, always or only when noise is detected.
- Measure on the user's recordings with and without background noise; keep suppression only where it lowers the word error rate, drop it if it hurts.
- If kept: the recording is cleaned of noise locally before it goes to the recognizer, with a setting to turn it off.

Current stage: PoC. Target stage: PoC.
Scope: the measurement and, if it helps, local noise suppression before dictation recognition.
Acceptance bar: on noisy recordings the word error rate drops; on clean recordings it does not get worse; dictation delay stays acceptable. If no suppressor passes, the change ends with the measurement written down and no code shipped.
Out of scope: voice command spotting audio; paid or cloud services; microphone hardware.
Interface: none (config file only).

## Capabilities

### Modified Capabilities

- `dictation`: optional local noise suppression before recognition.

## Impact

- Audio pipeline before transcription, a new local dependency or model, configuration, accuracy bench.

## Open points

- Suppression may worsen recognition — proposed: measure first; ship only if it helps on noisy and does not hurt on clean audio.
- Which suppressor (DeepFilterNet, RNNoise, Apple voice processing) — proposed: compare at explore.
- Added delay per recording — proposed: budget it in the design.
- Where to get noisy test recordings — proposed: the user records some with background noise, plus clean archive clips mixed with noise.

## Release

SemVer: MINOR — new optional audio step (or no release if the measurement drops it).
Target version: 0.52.0.
