## Why

When the user says the start command ("старт-старт", said twice), the overlay and the start cue come too late: the user feels the delay. Recording itself starts earlier, so no speech is lost, but the late overlay and sound make a worse experience. The user's estimate: half a second at best, a second and a half or more at worst. The app's own log of 2026-10-27 confirms it: five voice starts took `total=594ms` to `total=1228ms`, split as

- `hear` (the keyword spotter waits for the utterance to end): 138–795 ms, the largest and most variable part;
- `confirm` (the speech recognition service for commands, Whisper small, confirms the utterance): 120–199 ms;
- `deliver`: 0–22 ms;
- `cue` (up to the start cue): 234–304 ms.

The goal: the overlay and the start cue appear right after the command is said — ideally 100–300 ms after its end, where the user barely feels a delay. Above that the delay is felt.

## What Changes

- Investigate where the time from the end of the spoken start command to the overlay and the start cue goes, part by part, on the user's machine and voice, and which parts can be cut.
- Try the ways the user named and any others the investigation finds, for example:
  - starting on the keyword spotter alone, without waiting for the Whisper confirmation, or confirming after the overlay is shown;
  - tuning the keyword spotter (and its voice activity detector) to decide sooner after the utterance ends;
  - showing the overlay and playing the cue faster in the app (the `cue` part alone is about 250 ms).
- Apply the ways that bring the start closer to the 100–300 ms goal without more false starts, and report the measured gain of each.

Current stage: PoC. Target stage: PoC.
Scope: the latency of a voice start, from the end of the spoken start command to the overlay and the start cue.
Acceptance bar: the investigation names each part of the delay with measurements; after the change, voice starts on the user's machine show the overlay and play the cue measurably sooner than the 594–1228 ms of 2026-10-27, as near 100–300 ms as the measurements allow, with no more false starts on ordinary speech than today.
Out of scope: the latency of other voice commands and of hotkey starts, except where the same fix speeds them up; dictation transcription speed.

## Capabilities

### Modified Capabilities

- `voice-control`: a voice start shows the overlay and plays its cue sooner after the start command is said.

## Impact

- The command spotting service (`command-spotting/`): when an utterance counts as ended and when its command is reported.
- The app's voice control pipeline, overlay and cue playback (`app/`).
- The start cue timing checks (`e2e-cues`) and their bounds.

## Open points

- Whether the Whisper confirmation can be dropped or moved after the overlay without more false starts: the statistics the change `log-voice-command-history` collects (how often Whisper corrects or rejects the keyword spotter) answer it. Proposed: measure on the user's recordings in this change and decide here; no hard dependency.
- What the `cue` part measures exactly and why it takes about 250 ms: check first, it may be the cheapest gain.
- Whether the double command ("старт-старт") itself adds `hear` time compared with a single word. Proposed: measure both, keep the double form unless the gain is large.
- The goal as a bound for `e2e-cues`. Proposed: tighten the bound to what the change reaches.

## Release

SemVer: MINOR — a visible change in voice control behavior.
Target version: 0.40.0 (proposed: the next minor after 0.39.0 of `log-voice-command-history`; take the next free minor at implementation time).
