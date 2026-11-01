## Why

The Russian streaming recognizer of the command spotting service (sherpa-onnx zipformer small ru, vosk) often writes a doubled "enter enter" as one glued word, and sometimes mangles a single "enter". The matcher reads the Russian text through the respelled command words, which match whole words only, so a glued or mangled form is no command at all. When the English recognizer hears nothing at the same moment — common with the user's pronunciation — the command is lost and the user stops the dictation with the hotkey, so the spoken "enter enter" lands in the pasted text.

Evidence from the dictation archive and the command history since 2026-10-29: of 301 dictations started by voice, 10 ended with the hotkey after a missed final command (almost always "enter enter"); in about half of them the Russian recognizer had written a glued form ("интернтер", "интерентер", "интернетор") while the English one heard nothing. The other half came from the service dropping audio under load, which this change leaves alone.

Forms seen in the command history (count in brackets):

- glued doubles: интернтер (4), интерентер (3), энтерентор (2), энтеррентор (2), интернетор, интеррентор, интерендер, интеррендер, энтернтер, интеринтер, энтер-энтер;
- a glued triple: энтеренторендрента;
- mangled singles: нтр (5), энтро (3), рентор (2), энтор, энтар, нтэр, тентер, рентра.

Ordinary words that hold the same letters and must stay plain speech: интересно, интересная, интерфейса, интервью, центр, контракт, контроллер, линтры.

## What Changes

- The glued doubles above count as "enter" said twice, so they take effect like a doubled command ("enter enter") — anywhere in an utterance, with no pause needed; the glued triple counts likewise.
- The mangled singles above count as "enter", under the same placement rules as "enter" itself (on its own, or at the end of an utterance).
- The forms are an explicit list of rows in the mishearings table, never a prefix or partial-word rule, so the ordinary words listed above keep doing nothing.

Current stage: PoC. Target stage: PoC.
Scope: Russian-recognizer mishearings of "enter", glued doubled and tripled forms included, as voice commands.
Acceptance bar: each listed form, written by the Russian recognizer, yields "press enter" (a glued double as a doubled command); each listed ordinary word yields no command; the existing commands behave as before.
Out of scope: the command spotting service dropping audio under load; a third, Whisper-based listener for utterances the streaming recognizers miss; mishearings of commands other than "enter".

## Capabilities

### Modified Capabilities

- `voice-control`: Russian mishearings of "enter", glued doubles included, are recognized as the command.

## Impact

- `voice-commands/src/lib.rs` — new rows in `MISHEARINGS` (Alias kind).
- The command spotting matcher (`command-spotting/src/matcher/`, `command-spotting/src/streaming/respell/mod.rs`) — a single glued word that stands for a doubled command.
- Tests of the matcher and of the voice commands table.

## Open points

- Whether a glued double maps to the existing doubled-command form or needs a mishearing kind of its own — proposed: reuse the doubled-command form, so it acts exactly like "enter enter".
- Whether "рентор" and "рентра" are safe enough as singles — proposed: keep them, both were seen only where "enter" was said.
- Target version — proposed 0.43.1 (PATCH).

## Release

SemVer: PATCH — more spoken forms of an existing command are recognized; no new behavior or setting.
Target version: 0.43.1.
