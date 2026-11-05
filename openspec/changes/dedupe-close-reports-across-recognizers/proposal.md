## Why

In the command spotting service, every utterance is decoded by two recognizers, English and Russian, and each reports the command it hears. When both hear the same command but time it apart, `doubles` in `command-spotting/src/streaming/decide/mod.rs` (the same commands and a gap under `DOUBLE_GAP_SECS`, 0.6 s) lets both reports through, and the app carries the command out twice. The service log of 2026-11-03/04 shows it in short utterances of one "enter": the Russian recognizer reports it at the gate's pause, and the English one, which stamps a word 0.65–1.1 s later than the Russian one, reports it again near or at the close (eleven such utterances in two days). The handover fix of `fix-missed-dictation-start-stop-commands` (released as v0.46.2) removed a case between utterances; inside one utterance the rule still allows it.

## What Changes

- A command one recognizer already reported in an utterance is not reported again by the other, however far apart the two stamp it: a recognizer's n-th finding of a command is a repeat when the utterance already holds n reports of it (both hear the same speech, so the n-th occurrence in one is the n-th in the other). The gap rule stays for a recognizer's own second word of a doubled form, and the recognizer that heard the whole word still speaks first.
- The log line of the utterance still shows what each recognizer heard, so the dedupe is visible.
- A session test with two scripted recognizers, the English one stamping the same "enter" about a second after the Russian one, expects one report; the same command said twice with a pause between, heard by both, still gives two.

Current stage: PoC. Target stage: PoC.
Scope: the report of `command-spotting/src/streaming/session/utterance.rs` and `doubles` in `decide/mod.rs`.
Acceptance bar: the new session tests pass; the existing session, close-order and voice-sample tests stay green; `just check` passes; the user's sessions 1–6 give one report per label.
Out of scope: the handover between utterances (done in v0.46.2); two recognizers reporting different commands of one chain (`settle-chain-tail-at-the-close`).
Interface: none.

## Capabilities

### Modified Capabilities

- `voice-control`: one utterance reports a command once, whichever recognizers heard it and however they time it.

## Impact

- `command-spotting/src/streaming/decide/mod.rs`, `command-spotting/src/streaming/session/utterance.rs`, `command-spotting/src/streaming/session/tests/`.

## Open points

Settled by the explore of 2026-11-05: the rule counts occurrences instead of comparing times, as the English recognizer's stamps lie beyond any gap that still separates two commands of one recognizer; it applies to every report of the utterance, not only the final ones, as the log shows the second report before the close as well as at it; the close-order rule stays. Target version: the next PATCH above `main` when the change runs (`kit version-check`).

## Release

SemVer: PATCH — a bug fix: one command carried out twice.
Target version: 0.46.3.
