## Why

In the command spotting service, every utterance is decoded by two recognizers, English and Russian, and each reports the command it hears. At the close of an utterance both finish with a final decode. When both hear the same command but time it differently — their findings overlap only partly, the Russian word ending before the English one starts or the other way round — `doubles` in `command-spotting/src/streaming/decide/mod.rs` (the same commands and a gap under `DOUBLE_GAP_SECS`) lets both reports through, and the app carries the command out twice. This showed in the service log of the user's session 1 while `fix-missed-dictation-start-stop-commands` was being fixed (released as v0.46.2): one labelled "start dictation", two reports from the two recognizers of one utterance. The handover fix of that change removed the case seen then, but the rule itself still allows it.

## What Changes

- The final reports of one utterance are deduped across its recognizers: a command one recognizer reported at the close is not reported again by the other when their findings overlap in time or lie within the double gap, whichever recognizer finished first.
- The log line of the utterance still shows what each recognizer heard, so the dedupe is visible.
- A session test with two scripted recognizers that finish the same command at partly overlapping times expects one report.

Current stage: PoC. Target stage: PoC.
Scope: `doubles` and the report of the close in `command-spotting/src/streaming/session/utterance.rs` and `decide/mod.rs`.
Acceptance bar: the new session test passes; the existing session, close-order and voice-sample tests stay green; `just check` passes; the user's sessions 1–6 give one report per label.
Out of scope: reports during the utterance (before the close), the handover between utterances (done in v0.46.2).
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: one utterance reports a command once, whichever recognizers heard it.

## Impact

- `command-spotting/src/streaming/decide/mod.rs`, `command-spotting/src/streaming/session/utterance.rs`, `command-spotting/src/streaming/session/tests/`.

## Open points

- The overlap rule: proposed "the same commands and the findings overlap in time, or their gap is under `DOUBLE_GAP_SECS`", applied to the final reports only.
- Whether the dedupe prefers the recognizer that heard the whole word (the close-order rule) or the first to finish: proposed the close-order rule, as today.
- Target version: the next PATCH above `main` when the change runs (`kit version-check`).

## Release

SemVer: PATCH — a bug fix: one command carried out twice.
Target version: 0.46.3.
