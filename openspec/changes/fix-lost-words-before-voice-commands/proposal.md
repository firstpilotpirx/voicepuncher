## Why

Voice control loses dictated words. Three dictations of 2026-11-13 (app 0.48.1) ended short; the dictation archive (`~/.voicepuncher/history/`) and the command history (`~/.voicepuncher/command-history/`) show two separate bugs.

![Dictation cut after «потому что»; «ну что нам мне это читать» lost](assets/c7a363e8fee6.png)
![Dictation cut after «потому что я»; «откуда её возьму» lost](assets/0e56886b8276.png)
![User said «Да, идём дальше»; only «Да» was pasted](assets/88a82feaee4a.png)

**Bug 1 — the dictation is cut about 0.8 s before the command phrase.** When command spotting reports a stop phrase ("press enter"), the app truncates the recording at the phrase start, but the kept audio ends about 0.8 s before the phrase start the service reported. Measured by aligning each dictation's audio with its command-history clip (loudness envelope cross-correlation, ncc ≥ 0.97):

| Dictation (history) | Command clip | Phrase start in clip | Dictation ends in clip | Lost |
|---|---|---|---|---|
| 2026-11-13T15-26-26.540Z | 15-26-25.729Z | 3.97 s | 3.16 s | «ну что нам мне это читать» |
| 2026-11-13T16-31-03.185Z | 16-31-18.963Z | 5.78 s | 4.96 s | «откуда её возьму» |
| 2026-11-13T16-38-34.153Z | 16-38-33.698Z («да идём дальше энтер») | 1.78 s | — (audio holds only «Старт. Да.») | «идём дальше» |
| 2026-11-13T12-00-17.598Z | 12-00-14.916Z («старт заново список энтер») | — | 0.71 s of audio left, outcome `no_text` | the whole phrase |

A short phrase said right before "энтер" falls fully into the gap and nothing is pasted. The recorder's `begin_with` already moves `started_at` back by the lead-in, so the offset comes from elsewhere — likely the time mapping between the command spotting stream (`heard.rs` `captured_at`, `kept_mic`) and the recorder's `started_at` (device latency or buffering).

**Bug 2 — false "press enter" inside longer words.** The Russian streaming recognizer reports "press enter" on the sound «энтер» inside longer words and fires before the word ends, stopping the dictation mid-sentence:

- «интернет» — clip 2026-11-13T16-31-18.963Z, reported at 5.78 s;
- «линтер» — clip 2026-11-12T20-17-16.737Z, reported at 0.98 s.

## What Changes

- A dictation stopped by a voice command keeps every word said before the command phrase; nothing before the phrase start is cut.
- "press enter" fires only when «энтер» / "enter" is said as a whole word, not as part of «интернет», «линтер» or similar words.

Current stage: PoC. Target stage: PoC.
Scope: the cut point of a dictation stopped by a voice command, and whole-word matching of the "press enter" command phrase.
Acceptance bar: red-then-green tests reproduce both bugs (the user's recordings above or synthesized speech): the words before "энтер" are in the pasted text, and «интернет» / «линтер» inside a dictation do not trigger "press enter"; a plain «энтер» still triggers it without a noticeably longer delay.
Out of scope: other false commands not seen here; recognition accuracy of the dictation itself.
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: the dictation keeps all speech up to the stop phrase; command phrases match whole words only.

## Impact

- `app/` recording cut (`truncate_at`, `cpal` recorder `started_at`) and the command spotting time mapping (`adapters/secondary/commandspotting/websocket/heard.rs`, `kept_mic.rs`).
- `command-spotting/` phrase matching for "press enter" (Russian recognizer).
- Tests: `voice_control.feature` scenarios and integration tests on the user's voice samples.

## Open points

- Where the 0.8 s offset comes from (device latency, the lead-in, the stream's time mapping): proposed — find it in design by measuring, then fix the mapping rather than adding a fixed margin.
- How to tell a whole word «энтер»: proposed — wait for the recognizer's next token or a short silence after «энтер» before reporting, and reject it when the token grows into a longer word; check the added delay against the voice start/stop budgets.
- Whether the two bugs should be split into two changes: proposed — one change, since both lose words before a command.

## Release

SemVer: PATCH — bug fix.
Target version: 0.49.1.
