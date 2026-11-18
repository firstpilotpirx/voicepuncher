## Why

Voice control loses dictated words. Three dictations of 2026-11-13 (app 0.48.1) ended short; the dictation archive and the command history show two bugs.

![Dictation cut after «потому что»; «ну что нам мне это читать» lost](assets/c7a363e8fee6.png)
![Dictation cut after «потому что я»; «откуда её возьму» lost](assets/0e56886b8276.png)
![User said «Да, идём дальше»; only «Да» was pasted](assets/88a82feaee4a.png)

**Bug 1 — the dictation is cut about 0.8 s before the command phrase.** Aligning each dictation's audio with its command-history clip shows the kept audio ends 0.81–0.82 s before the reported phrase start («ну что нам мне это читать», «откуда её возьму», «идём дальше» lost); a phrase said right before "энтер" can vanish whole (12:00:17, outcome `no_text`).

**Bug 2 — false "press enter" inside longer words.** The Russian recognizer reports "press enter" on «интер» before it grows into «интернет» (16:31:18), and on «линтр» (linter, 2026-11-12 20:17:16).

## What Changes

- A dictation stopped by a voice command keeps every word said before the command phrase.
- "press enter" fires only when «энтер» / "enter" is said as a whole word, not inside «интернет», «линтер» or similar words.

Current stage: PoC. Target stage: PoC.
Scope: the cut point of a dictation stopped by a voice command, and whole-word matching of "press enter".
Acceptance bar: red-then-green tests reproduce both bugs: the words before "энтер" are in the pasted text, and «интернет» / «линтер» inside a dictation do not trigger "press enter"; a plain «энтер» still triggers it without a longer delay.
Out of scope: other false commands; recognition accuracy of the dictation itself.
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: the dictation keeps all speech up to the stop phrase; command phrases match whole words only.

## Impact

- `app/` command spotting time mapping (`adapters/secondary/commandspotting/websocket/`).
- `command-spotting/` respelling and the at-once rule for "press enter".

## Release

SemVer: PATCH — bug fix.
Target version: 0.49.3.
