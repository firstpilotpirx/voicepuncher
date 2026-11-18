## Why

When the user speaks fast, dictation often cuts phrases and gets words wrong; slow, clear speech is recognized fine. The suspected cause is how the recording is cut into chunks (chunk seams, voice activity detection), not the model. The cause is not known yet: research is mandatory before any fix.

## What Changes

- Research first: find why fast speech loses or breaks phrases, using the logs and the dictation archive (`~/.voicepuncher/history`, ~3000 entries from the last month with audio, chunk texts and timings).
- Then fix the found cause, so fast speech keeps every phrase and its words whole.

Current stage: PoC. Target stage: PoC.
Scope: the research (where words are lost: at chunk seams, in VAD cuts, in the model, in refinement) and the fix of the found cause.
Acceptance bar: on the user's own fast-speech recordings, no phrase is dropped or cut at a seam, and the word error rate is lower than today; the research result is written in the design.
Out of scope: the dictation language and glossary; noise suppression; a new model.
Interface: none.

## Capabilities

### Modified Capabilities

- `dictation`: chunking of fast speech keeps every phrase whole.

## Impact

- Chunking/seams and VAD code, the seam E2E tests (`e2e-seams`), possibly the refinement step.

## Open points

- The cause may be elsewhere (model on fast speech, refinement dropping words) — proposed: research decides; if the model itself is the cause, record it and file a follow-up instead of tuning here.
- Which archive entries count as "fast speech" — proposed: pick by words per second from the archive, plus a few new recordings by the user.

## Release

SemVer: PATCH — bug fix.
Target version: 0.51.1.
