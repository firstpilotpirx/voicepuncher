## Context

The Russian streaming recognizer writes an "enter enter" said quickly as one glued word and a quiet "enter" as a mangled one. Its words are respelled in English by reading them as command sounds one after another (`command-spotting/src/streaming/respell/`); a word the sounds cannot read whole stays Cyrillic and is no command. Re-reading the command history since 2026-10-29: the listed forms the sounds already read (интерентер, энтерентор, интерендер, интеринтер, энтор, энтар) were reported, by either recognizer; the lost ones (интернтер, интеррентор, интеррендер, нтр, рентор) are those the sounds cannot read — a second half without its vowel, a leading "р", or a hyphen between the halves.

## Decisions

### Alias rows in the mishearings table, read before the command sounds

A new mishearing kind, `Alias`, in the voice commands table (`voice-commands/src/lib.rs`): one row per word the Russian recognizer writes for a command said once or twice, its `heard` the one Cyrillic word and its `means` the command once ("enter") or twice ("enter enter"). The respelling reads an alias row first: the whole word, lowercased and with "ё" read as "е", must equal the row's word; then it becomes the one-word short form of each command in `means`, so a glued double becomes "enter enter" and takes the doubled-command path as it is (anywhere, no pause, reported once), and a mangled single becomes "enter" and takes its placement rules. Only when no row matches are the command sounds read, as before.

Rows only for the forms the sounds cannot read; a form the sounds already read gets no row and a test that pins it. The glued triple ("энтеренторендрента") means "enter" twice: a doubled command is reported once, so a third "enter" would add nothing.

Alternatives rejected:
- Loosening the "enter" sound pattern (an optional vowel, a leading "р"): would read "интернет" and "центр"-like words as commands; a whole-word list cannot.
- A new placement in the matcher for a single glued word: duplicates the doubled-command path; respelling the word to its two words reaches it unchanged.

### A hyphen inside a Cyrillic word is dropped before its sounds are read

"энтер-энтер" reaches the respelling as one word with a hyphen, which no sound reads, so it stays Cyrillic and the matcher never sees "enter enter". The respelling keeps only the letters of a word before reading it, so the word reads as its two halves. An ordinary hyphenated word ("вент-коллектор") still reads as no command, as its letters are no command sounds.

## Test levels

| Scenario | Level |
|---|---|
| A Russian-accented stop is heard as stop | E2E `.feature` (existing) |
| A Russian word that is no command stays speech | E2E `.feature` (existing) |
| A glued enter enter written by the Russian recognizer counts as the doubled command | unit (`command-spotting/src/streaming/session/tests/`, scripted recognizers) |
| A mangled enter written by the Russian recognizer counts as enter | unit (`command-spotting/src/streaming/session/tests/`, scripted recognizers) |
| Every glued or mangled enter from the history reads as the command | unit (`command-spotting/src/streaming/respell/tests.rs`) |
| An ordinary Russian word with the letters of enter stays speech | unit (`command-spotting/src/streaming/respell/tests.rs`) |

The table's own rules (an alias row hears one word and means one command once or twice) are unit tests in `voice-commands/src/tests.rs`. No E2E scenario synthesizes these forms: macOS `say` cannot be made to produce the recognizer's mishearings; the real recordings behind them are in the command history, not in the voice samples.

## Risks

- A listed single ("рентор", "рентра", "тентер") said as part of ordinary speech would press Return; each was seen only where "enter" was said, and a single counts only alone or at the end of an utterance.
