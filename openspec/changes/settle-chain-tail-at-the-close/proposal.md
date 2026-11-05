## Why

A chain of commands said in one breath ("that is all stop enter start") is one utterance of the command spotting service, decoded by its English and Russian recognizers. The service reports a chain as soon as a recognizer has decoded it, and the English recognizer emits each word only once more audio came after it, so the last command of a chain often shows up only after the commands before it were reported (the late emits that the flush at the gate's pause and the close silence of `fix-missed-dictation-start-stop-commands`, released as v0.46.2, were added for). The service then judges that last word on its own, among the words said after the last report, and a word that is a command only after another one — "start" after "stop", accepted while dictating only as the end of a chain — matches nothing: the chain loses its tail. On the user's recording session-4/5 ("that is all stop enter start") the bench in real time lost "start" in 3 runs of 3, although both recognizers' final texts hold the whole chain; a chain whose tail the Russian recognizer hears glued to the word before it ("энтерстарт", session-4/6 and 4/8) is reported whole and keeps it.

## What Changes

- A command said right after a reported chain continues it: the service matches the chain over every word of the utterance and reports the commands said after the last report, so the tail is reported once, by the first recognizer to decode it, in the order said, with "start" last accepted as the chain's end as today.
- The loss is measured before and after on the user's recorded chains (`just streaming-bench --recorded`, sessions 1–6, session 4 three times in real time), in the design.
- A session test with scripted recognizers where the English one emits the chain's last word only after the commands before it were reported expects the last command reported.

Current stage: PoC. Target stage: PoC.
Scope: the decision of `command-spotting/src/streaming/decide/mod.rs` and its call in `command-spotting/src/streaming/session/utterance.rs`; the decide unit tests, the session tests of `command-spotting/src/streaming/session/tests/`, and an E2E scenario over the user's recorded chains in `e2e-test/tests/features/voice_control.feature`.
Acceptance bar: no chain tail lost on the user's recorded chains in three real-time bench runs of session 4, nor on sessions 1–6; the new tests pass and the existing chain, close-order and voice-sample tests stay green; `just check` passes; `full-check` with the voice control regression scenarios passes.
Out of scope: single commands, the gate, the bench itself and its `--fast` mode (`fast-bench-reproduces-app-timing`), a command both recognizers report doubled with different timings (`dedupe-close-reports-across-recognizers`), a new E2E timing bound for chains (none exists; the bench's delay per clip is the measure).
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: the last command of a chain, decoded after the commands before it were reported, is still carried out, in order.

## Impact

- `command-spotting/src/streaming/decide/mod.rs`, `command-spotting/src/streaming/decide/tests.rs`, `command-spotting/src/streaming/session/utterance.rs`, `command-spotting/src/streaming/session/tests/`, `e2e-test/tests/features/voice_control.feature`, `e2e-test/tests/steps/voice_control/`.

## Open points

- How the tail is matched: proposed the chain matched over every word of the utterance, the commands whose span ended by the last report dropped from the finding; the alternative, accepting a lone fresh word after the last reported command, needs the matcher to learn the report.
- Target version: the next PATCH above `main` when the change runs (`kit version-check`).

## Release

SemVer: PATCH — a bug fix: a command said is not carried out.
Target version: 0.46.3.
