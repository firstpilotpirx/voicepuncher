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
- The forms the command sounds do not already read are an explicit list of rows in the mishearings table, never a prefix or partial-word rule, so the ordinary words listed above keep doing nothing; the forms the sounds already read (интерентер, энтерентор, интерендер, интеринтер, энтор, энтар) get no row and are pinned by tests.
- A hyphen inside a word the Russian recognizer writes ("энтер-энтер") is dropped before its sounds are read, so the word reads as the two words it is.
- Related work met in apply: the `trufflehog` pin of the `justfile` follows the installed 3.99.8 (Homebrew moved past 3.99.6, and `_require-tools` refused every `just test`); `speed-up-check-runtime` makes the same bump.
- Related work met in full-check: Homebrew had upgraded `ggml` to 0.28.0 and `libomp` to 23.1.4 and dropped the pinned versions, so `just engines` refused to stage; `app/packaging/engines.lock` and README § Engines now pin the installed versions with their checksums, and the three formulae are pinned with `brew pin` (`e2e-bundle` proves the engines still run on Metal).

Current stage: PoC. Target stage: PoC.
Scope: Russian-recognizer mishearings of "enter", glued doubled and tripled forms included, as voice commands.
Acceptance bar: each listed form, written by the Russian recognizer, yields "press enter" (a glued double as a doubled command); each listed ordinary word yields no command; the existing commands behave as before.
Out of scope: the command spotting service dropping audio under load; a third, Whisper-based listener for utterances the streaming recognizers miss; mishearings of commands other than "enter".

## Capabilities

### Modified Capabilities

- `voice-control`: Russian mishearings of "enter", glued doubles included, are recognized as the command.

## Impact

- `voice-commands/src/lib.rs` — new rows in `MISHEARINGS` (Alias kind: a word the Russian recognizer writes for a command said once or twice).
- The respelling of the Russian recognizer's words (`command-spotting/src/streaming/respell/mod.rs`) — an alias row is read before the command sounds; a hyphen is dropped. The matcher stays as it is.
- Tests of the respelling, of the streaming session and of the voice commands table.

## Decisions (explore, 2026-11-04)

- A glued double maps to the existing doubled-command form: its row means "enter" twice, so the respelled word reads "enter enter" and acts exactly like it; no mishearing kind beyond the alias row.
- "рентор" and "рентра" stay as singles: both were seen only where "enter" was said (the command history since 2026-10-29 holds no other use).
- Evidence re-read in the command history: of the listed forms, the English recognizer caught most at the same moment; the ones lost (интернтер, интеррентор, интеррендер, нтр, рентор) are exactly those the command sounds do not read.
- Target version 0.46.1 (PATCH): `main` is at 0.46.0.

## Release

SemVer: PATCH — more spoken forms of an existing command are recognized; no new behavior or setting.
Target version: 0.46.1.
