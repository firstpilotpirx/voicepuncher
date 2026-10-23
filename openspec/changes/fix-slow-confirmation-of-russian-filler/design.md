# Design

## Context

See proposal.md § Why. Every confirmation of the command spotting service goes through `UtteranceTranscriber::transcribe` (`command-spotting/src/confirm.rs`): a copy of the speech recognition service for commands decodes the utterance until its *decoding budget* runs out, then the request is closed and, when the copy answers its health check, the utterance counts as heard as nothing. The budget comes from `Pace` (`confirm/pace.rs`, from `voice-commands-under-load`): twice the slowest confirmation of the last 10 seconds, at least 1 s, never past the 3 s deadline. `settle` notes in the pace both the time of every answer and, for a confirmation cut short, the budget it was cut at.

## The cause of the 2.116 s confirmation

The pace takes speech that makes speech recognition repeat itself for machine load, and so the budget raises itself:

1. A Russian clip that gets stuck repeating decodes whisper's whole 219-token window: 1.7–1.8 s on an idle copy (`fast-confirmation-of-repeating-russian-speech`, design.md). With the budget at its 1 s floor it is cut short at 1 s, and `settle` notes 1 s.
2. Twice 1 s gives the next confirmation a 2 s budget for 10 seconds. The next clip stuck repeating now decodes to its end and is answered in 1.7–1.8 s, which `settle` notes as an answer.
3. Twice 1.8 s passes the deadline, so the budget is 3 s for the next 10 seconds: every further repetition loop is answered in full, in 1.7–1.8 s on an idle machine and longer beside a heavy stage.

The scenario's eight Russian phrases give several clips stuck repeating within 10 seconds, so step 3 is reached in every run. The failing run shows it: on a 1 s budget no confirmation could take more than about 1.05 s (the budget and the health check), yet the other 58 confirmations of that run took up to 1.8 s, the time of a loop decoded in full, and the failing one, 2.116 s, is such a loop decoded beside the load of another run's heavy stage (load average 14–20). The scenario passed until then because an idle copy ends a loop in under 1.85 s, a margin of 0.15 s to the bound that a little load takes away.

Measured beside this change on 2026-10-23 (one `whisper-server` with the small model and the flags of `start-model-servers`, the scenario's eight phrases each sent whole, 5 rounds idle and 5 rounds beside the large model decoding on Metal in two loops and 8 busy CPU loops): no whole phrase got stuck repeating (12–27 characters of text each), ordinary answers took 0.14–0.7 s idle and 0.19–0.7 s under that load, and single answers stalled for 1.7–10 s in both phases while other sessions' runs loaded the machine. A stall of an ordinary answer is cut short like a loop, so with the old pace it too lengthened the budget (step 1). The clips that get stuck are the cuts of the keyword spotter, not whole phrases: which cut repeats depends on the exact audio (`fast-confirmation-of-repeating-russian-speech`).

GPU contention alone does not explain the failure: it slows a loop from 1.8 s to 2.1 s, but a loop reaches its end only because the budget had raised itself first. So the lock class of `e2e-resilience` stays light (the preparation's decision for that case does not apply).

## Goals / Non-Goals

**Goals:** a confirmation cut short, and an answer of speech stuck repeating, never lengthen the budget; on an idle machine every confirmation then ends in about 1 s at most; ordinary answers that come back slowly under load still lengthen it as before.

**Non-Goals:** the floor, the window, the slack and the deadline of the pace; a token limit (whisper-server 1.9.5 takes none per request; retries are off already, `temperature_inc` 0); the lock class of `e2e-resilience`; the bound of the scenario.

## Decisions

**D1. A confirmation cut short sets no pace.** `settle` no longer notes the budget of a confirmation cut short. A copy still decoding at its budget is either stuck repeating or slowed by load, and nothing tells the two apart; only an answer shows how fast the copy decodes ordinary speech. Under load, the ordinary answers that come back within the budget (0.19–0.7 s measured, 0.5–1.2 s in `voice-commands-under-load`) raise it as before.

Alternatives: keep noting a cut short but at less than its budget (still raises the budget after a loop, only slower); a token limit from the audio length (whisper-server takes no such field); move `e2e-resilience` to the heavy class (the budget would still raise itself on an idle machine and leave a 0.15 s margin).

**D2. An answer stuck repeating sets no pace.** An answer whose text is longer than 120 characters is speech recognition stuck repeating: a confirmation holds at most 2.5 s of speech, ordinary answers are 1–12 tokens (12–40 characters measured), while a loop is 219 tokens. Such an answer is still returned to the engine, which hears no command in it; only the pace ignores it. Without D2, a loop answered within a budget that slow ordinary answers had raised (1.6 s under a 1.9 s budget) would raise the budget to the deadline.

Alternatives: count words (a loop such as "E-e-e-e-…" is one word); time per token (verbose JSON without token counts per segment, and the cut-short case still has no answer).

**D3. The scenario prints its confirmation times when it passes.** The step "every confirmation of that speech is answered in under 2 seconds" writes the slowest time, the number of confirmations cut short and every time to the stage log, so each run under load measures the margin to the bound and the acceptance bar (three `full-check` runs of other changes) can be read from their logs.

## Risks / Trade-offs

- [A machine so loaded that every ordinary confirmation takes over 1 s never raises the budget, and commands are cut short] → the measured ordinary answers under a heavy stage stay under 0.7 s; the ones that stall for seconds were cut short before this change too, and a raised budget only waited longer for them. The real-service tests of `placed_commands` raise their own floor (`fix-placed-commands-real-service-under-load`).
- [A long ordinary answer over 120 characters] → a confirmation holds at most 2.5 s of speech, about 40 characters spoken fast; such an answer still reaches the engine.
- [The fix is proven by unit tests with a scripted copy, the E2E scenario only by its pass rate] → D3 makes every run a measurement; § Results gives the run of this change under load.

## Tests

| Scenario | Level |
|---|---|
| Russian speech that makes speech recognition repeat itself | E2E `local_services.feature` (existing; prints its times, D3) |
| A command right after such speech is carried out | E2E `local_services.feature` (existing) |
| A confirmation still decoding after 1 second is cut short | unit, `confirm/tests/under_load.rs` (existing) |
| A confirmation decoding slowly under load is heard in full | unit, `confirm/tests/under_load.rs` (existing) |
| The budget never passes the deadline | unit, `confirm/tests/under_load.rs` and `confirm/pace.rs` (existing) |
| Speech cut short does not lengthen the budget | unit, `confirm/tests/under_load.rs` |
| An answer stuck repeating does not lengthen the budget | unit, `confirm/tests/under_load.rs` |

## Results

To be filled in by the apply stage.
