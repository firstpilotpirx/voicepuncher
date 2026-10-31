## Why

Since 0.42.0 the default streaming recognizer made voice commands slower and harder to trigger: voice starts took 0.7–1.2 s from the end of the words to the start cue (0.3–0.5 s with the keyword spotter, above the promised 0.7 s), "stop" longer, and "start" had to be said loudly. Traces find the command ~0.2 s after its words but deliver it ~1 s later: the English recognizer barely keeps up and every frame waits for it.

## What Changes

- A replay bench over the user's command history and voice samples measures wall-clock latency and command accuracy, before and after each step.
- Each recognizer decodes on its own; the Russian one no longer waits for the English one.
- The English recognizer decodes faster: more threads, performance cores, chunk size picked on the bench.
- A command said alone reports at the pause after it; an utterance closes with a shorter tail.
- Command words become recognizer hotwords when the bench shows a gain; mishearings from the user's history join the command forms.
- The voice gate hears quieter speech; the history also keeps utterances with no command found.

Current stage: PoC. Target stage: PoC.
Scope: the streaming recognizer in `command-spotting/`, the voice command forms, the bench, its history entries.
Acceptance bar: on the bench, start and stop reported within 0.3 s of the end of their words (median; 0.2 s is the stretch goal, bounded by the models' own chunk latency), command accuracy on the user's recordings at least 89 % (the measured share less the run-to-run spread); `just full-check` passes.
Out of scope: the keyword spotter (kept as fallback), a Whisper fallback after the utterance (filed separately if the bench shows enough misses), dictation.

## Capabilities

### Modified Capabilities

- `voice-control`: tighter start-cue bound and a bound for "stop" with the streaming recognizer, independent recognizers, shorter utterance tail, quieter speech heard, utterances without a command kept.

## Impact

`command-spotting/`, `voice-commands/`, the bench, `docs/ubiquitous-language.md`.

## Release

SemVer: PATCH — fixes latency and accuracy of an existing capability. Target version: 0.42.1.
