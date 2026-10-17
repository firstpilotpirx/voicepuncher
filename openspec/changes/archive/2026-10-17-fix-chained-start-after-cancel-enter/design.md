# Design

## Context

A running utterance is heard several times (`command-spotting/src/engine/confirming.rs`): a head check, a window check of its last 2.5 s every second once it runs 2 s, then the closed segment, then a tail check. Each answer goes through `place`, which reports the commands the matcher finds and moves the utterance's `reported_until` to the end of their last word; a later answer whose found commands start before `reported_until` is dropped whole, so no command is carried out twice.

Measured on ports of their own (speech recognition for commands on `ggml-small.bin`, the user's session-4 at 32.41 s, while dictating):

| build | window 1.00–3.50 s (`main`: 1.10–3.60 s) | segment | carried out |
|---|---|---|---|
| `main`, 5 runs | "Nevermind, cancel, enter, start." | same | cancel, enter, start |
| `detect-quiet-clear-all-text`, 2 runs | "Nevermind, cancel, enter style." | "Nevermind, cancel, enter, start." | cancel, enter |

The window is placed from the utterance's onset; the other branch's detector finds the onset 0.1 s earlier, so the window ends inside "start". The window reports "cancel, enter" (until 3.08 s); the segment's chain "cancel, enter, start" starts at 1.96 s, before 3.08 s, and is dropped with its "start".

## Decisions

### D1. A later answer reports the rest of a chain, not nothing

When the commands an answer finds start before `reported_until`, `place` keeps those whose words lie after it and reports them, in order, as one event placed by their own words; when none is left it reports nothing, as today. A command lies after `reported_until` when the middle of its words does: word times differ by up to about 0.1 s between two clips of the same speech, so "enter", which ended the earlier report, is not counted again, and "start", which follows it, is. `Found` gains the word range of each of its commands, so `place` can cut the chain at a command boundary.

The rest keeps the chain's placement; the matcher found the chain as a whole, so the states allow it after the reported commands. The rule also covers a tail check or later window.

**Rejected:** holding a window's command whose last word ends near the window's cut — it delays every chain a running utterance reports until the segment closes and adds one more tuned constant; moving the window or the detector — any fixed cut ends inside some word of some utterance.

### D2. No change to what is heard

The clips, the requests, the models and the matcher's chains stay as they are; only which of the found commands are reported changes. A form that passes today passes the same way, since an answer whose commands start at or after `reported_until` is reported whole as before.

## Test plan

| Spec scenario | Level | Test |
|---|---|---|
| The end of a chain heard only when the utterance ends still counts | command spotting integration, plain Rust (scripted hearing: window "Nevermind, cancel, enter style.", segment "Nevermind, cancel, enter, start.") | `command-spotting/tests/engine_it.rs` |
| same, the user's recording | adapter integration with the user's recordings | `app/tests/voice_forms_it.rs` (unchanged; run with this branch's service, and with `detect-quiet-clear-all-text`'s detector merged in by that change) |
| Word ranges of each command of a chain | unit | `command-spotting/src/matcher/placed.rs` tests |
| Stop enter pastes and sends; Stop enter start sends and starts the next dictation; Cancel clear discards and clears; A chain the states do not allow is no command | E2E `.feature`, unchanged | `e2e-test/tests/features/voice_control.feature` |

No `.feature` scenario: what a check hears while the utterance runs cannot be set from synthesized speech; the chain scenarios already in `voice_control.feature` keep covering chains end to end.
