## Why

After "stop" the user waits 1.0–1.5 s for the paste (log of 2026-11-01: `drain` 1.0–1.45 s of `total`). Two defaults cause it. Chunks are cut every 30 s, so a typical dictation (10–28 s) is sent as one piece only after "stop". And the language is `auto`: language detection is an extra encoder pass, measured at +0.45–0.55 s per request on the user's own recordings, while the user always speaks Russian, with English words inside.

Measured on the running service (10 recordings, median, fixed language): a 10 s piece takes 0.63 s, a 30 s piece 1.16 s; below 7 s the time stays at ~0.55 s.

## What Changes

- The default chunk threshold becomes 10 s instead of 30 s: chunks are transcribed while the user goes on speaking, so "stop" waits only for the last piece of at most 10 s.
- The default spoken language becomes Russian instead of automatic detection; forced Russian kept mixed Russian-English recordings word for word and still transcribed English speech in Latin letters.
- The settings window's Language choice offers only Russian, English and Detect automatically; a language the file sets outside them is still shown and kept.

Current stage: PoC. Target stage: PoC.
Scope: the two defaults, the default configuration file, the Language choice of the settings window.
Acceptance bar: with the defaults, stop → paste of a 20–30 s dictation is at most ~0.8 s in the log (`stop→paste: total`); the chunk seam stages and the 100-recording accuracy stage of `just full-check` pass.
Out of scope: how a cut point is picked and how seams are merged (unchanged), the "Languages to expect" list, voice command recognition, rewriting existing configuration files (the user's own already says `ru`).

## Capabilities

### Modified Capabilities

- `speech-recognition`: Russian is the default spoken language; the default chunk threshold is 10 s and stopping waits only for the last chunk.
- `settings-window`: the Language choice offers Russian, English and automatic detection.

## Impact

`app/src/config/transcription/`, the default configuration file, `app/src/adapters/primary/settings/macos/values/`, their tests, the E2E steps that relied on the `auto` default, `docs/ubiquitous-language.md` if a term changes.

## Release

SemVer: MINOR — changes two defaults the user sees and narrows a settings choice. Target version: 0.43.0.
