## Why

A chain of commands said in one breath ("press enter press enter paste text") is one utterance of the command spotting service. The English recognizer emits the last word of the chain only once more audio came after it (its late emits, which the flush at the gate's pause and the close silence of `fix-missed-dictation-start-stop-commands`, released as v0.46.2, were added for). In the service logs of that change the last command of a chain was sometimes reported only after the gate's close, later than the app's cue bound, or not at all when the utterance ended before the emit: the chain lost its tail.

## What Changes

- The last command of a chain is settled at the close: when the English recognizer has not emitted it by the final decode, the Russian recognizer's reading of the same words or the flushed silence settles it, so the chain's tail is reported once and within the cue bound.
- The loss is measured first: on the user's recordings (`~/.voicepuncher/voice-samples/`, sessions 1–6) and on the synthesized chains of the E2E scenarios, the count of chain tails reported late or lost, before and after the change, in the design.
- A session test with scripted recognizers where the English one emits the chain's last word only at the final decode expects the last command reported at the close.

Current stage: PoC. Target stage: PoC.
Scope: the close and the flush of `command-spotting/src/streaming/session/utterance.rs`, `decide/mod.rs`, the chain tests of `command-spotting/tests/it/streaming/` and the E2E chain scenarios.
Acceptance bar: no chain tail lost on the user's recordings and the synthesized chains; the chain's last cue within the E2E bound; `just check` passes; `full-check` with the voice control regression scenarios passes.
Out of scope: single commands, the gate, the bench.
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: the last command of a chain is reported at the close when a recognizer emits it late.

## Impact

- `command-spotting/src/streaming/session/utterance.rs`, `command-spotting/src/streaming/decide/mod.rs`, `command-spotting/tests/it/streaming/`, `e2e-test/tests/features/voice_control.feature`.

## Open points

- How the Russian recognizer settles an English last word: proposed by the respelled form at the end of its text, as the close-order rule does for a single word.
- Whether a longer flush at the close replaces the Russian fallback: proposed no, the flush is already 1 s.
- Target version: the next PATCH above `main` when the change runs (`kit version-check`).

## Release

SemVer: PATCH — a bug fix: a command said is not carried out.
Target version: 0.46.3.
