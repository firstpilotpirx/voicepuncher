## Why

The user still has to say "энтер" twice too often (app 0.49.1, after `fix-glued-enter-at-utterance-end`). In the command history of 2026-11-16..17 about 190 enters worked, and in 5 utterances Whisper hears an "enter" at the very end that command spotting never reported:

| Clip (`~/.voicepuncher/command-history/`) | Whisper hears | Russian recognizer | English recognizer |
|---|---|---|---|
| 2026-11-17T11-06-28.463Z-53022-6-2293 | "…всей этой фигни. Enter, enter." (0.44 s) | "…всей этой фигни" | "" |
| 2026-11-16T08-13-55.930Z-53022-6-227 | "Там отмечать, Enter." | "там отмечать энтерф" | "Tamat Michael" |
| 2026-11-17T11-05-24.182Z-53022-6-2278 | "Как это мэчится? Enter." | "как это мяч р" | "Kakatoshim" |
| 2026-11-17T11-08-10.776Z-53022-6-2303 | "…предложи мне, что получится, энтер." | "…что получится" | stuck at 4.7 s |

Each "enter" is fast (0.3–0.45 s) and glued to the last word. The Russian model writes a fragment or nothing; the English model, after seconds of Russian, writes nothing. Replaying the last second of speech alone heard most of them. A fifth case (dictation 2026-11-16T11-47-15.697Z) pressed Return but pasted "…вообще? Enter! вообще.": the command word stayed in the text.

## What Changes

- **Second look:** when an utterance closes, both recognizers also decode its last second of speech in fresh streams; when the utterance itself reported no command, a command heard there is reported.
- **Any "энтер…" ending an utterance** is "enter": a last word that starts with the sounds of "энтер" ("энтерф", "энтервь") counts, not only the listed aliases.
- **Hotwords:** both recognizers are biased towards "enter"/"энтер" (contextual biasing of the decoder).
- **No "Enter" in the pasted text:** a dictation ended by a command drops the command words at its end even when a repeated word follows them.

Current stage: PoC. Target stage: PoC.
Scope: command spotting at the end of an utterance; the trailing command words of a dictation's text.
Acceptance bar: the five clips above report "press enter" in the streaming bench (or the merged text has no "Enter"); the ordinary-words tests stay green; `just check` passes.
Out of scope: other commands' accuracy; the cut point of a stopped dictation (`fix-lost-words-before-voice-commands`); "интер…" prefixes ("интересно", "интернет" are ordinary words); a Whisper fallback.
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: a second look at an utterance's last second; an "энтер…" word ending an utterance counts; the pasted text drops command words followed by a repeated word.

## Impact

- `command-spotting/src/streaming/session/` (second look), `command-spotting/src/streaming/respell/` (prefix), `command-spotting/src/streaming/sherpa/` (hotwords), `app/src/core/domain/command_words/` (trailing words)

## Release

SemVer: PATCH — bug fix.
Target version: 0.49.2.
