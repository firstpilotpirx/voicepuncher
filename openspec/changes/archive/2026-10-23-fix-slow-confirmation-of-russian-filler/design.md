# Design

## Context

See proposal.md § Why. Every confirmation of the command spotting service goes through `UtteranceTranscriber::transcribe` (`command-spotting/src/confirm.rs`): a copy of the speech recognition service for commands decodes the utterance until its *decoding budget* runs out, then the request is closed and, when the copy answers its health check, the utterance counts as heard as nothing. The budget comes from `Pace` (`confirm/pace.rs`, from `voice-commands-under-load`): twice the slowest confirmation of the last 10 seconds, at least 1 s, never past the 3 s deadline. `settle` notes in the pace both the time of every answer and, for a confirmation cut short, the budget it was cut at.

## The cause of the 2.116 s confirmation

The pace takes speech that makes speech recognition repeat itself for machine load, and so the budget raises itself:

1. A Russian clip that gets stuck repeating decodes whisper's whole 219-token window: 1.7–1.8 s on an idle copy (`fast-confirmation-of-repeating-russian-speech`, design.md). With the budget at its 1 s floor it is cut short at 1 s, and `settle` notes 1 s.
2. Twice 1 s gives the next confirmation a 2 s budget for 10 seconds. The next clip stuck repeating now decodes to its end and is answered in 1.7–1.8 s, which `settle` notes as an answer.
3. Twice 1.8 s passes the deadline, so the budget is 3 s for the next 10 seconds: every further repetition loop is answered in full, in 1.7–1.8 s on an idle machine and longer beside a heavy stage.

The scenario's eight Russian phrases give several clips stuck repeating within 10 seconds, so step 3 is reached in every run. The failing run shows it: on a 1 s budget no confirmation could take more than about 1.05 s (the budget and the health check), yet the other 58 confirmations of that run took up to 1.8 s, the time of a loop decoded in full, and the failing one, 2.116 s, is such a loop decoded beside the load of another run's heavy stage (load average 14–20). The scenario passed until then because an idle copy ends a loop in under 1.85 s, a margin of 0.15 s to the bound that a little load takes away.

Measured beside this change on 2026-10-23 (one `whisper-server` with the small model and the flags of `start-model-servers`, the scenario's eight phrases each sent whole, 5 rounds idle and 5 rounds beside the large model decoding on Metal in two loops and 8 busy CPU loops): no whole phrase got stuck repeating (12–27 characters of text each), ordinary answers took 0.14–0.7 s idle and 0.19–0.7 s under that load, and single answers stalled for 1.7–10 s in both phases while other sessions' runs loaded the machine. A stall of an ordinary answer is cut short like a loop, so with the old pace it too doubled the budget (step 1). The clips that get stuck are the cuts of the keyword spotter, not whole phrases: which cut repeats depends on the exact audio (`fast-confirmation-of-repeating-russian-speech`).

GPU contention alone does not explain the failure: it slows a loop from 1.8 s to 2.1 s, but a loop reaches its end only because the budget had raised itself first. So the lock class of `e2e-resilience` stays light (the preparation's decision for that case does not apply).

## Goals / Non-Goals

**Goals:** a confirmation cut short lengthens the budget to 1.5 s at most, an answer of speech stuck repeating never lengthens it; on an idle machine every confirmation then ends in about 1.55 s at most; ordinary answers that come back slowly under load still lengthen it as before.

**Non-Goals:** the floor, the window, the slack and the deadline of the pace (only what a confirmation cut short notes changes); a token limit (whisper-server 1.9.5 takes none per request; retries are off already, `temperature_inc` 0); the lock class of `e2e-resilience`; the bound of the scenario.

## Decisions

**D1. A confirmation cut short sets the budget to 1.5 s, never more.** `settle` no longer notes the budget a confirmation was cut at; `Pace::note_cut_short` notes a fixed 0.75 s instead, so the budget after any number of confirmations cut short, at any budget, is 1.5 s for the window (twice 0.75 s), unless ordinary answers ask for more. A copy still decoding at its budget is either stuck repeating or slowed by load, and nothing tells the two apart; 1.5 s serves both: a copy slowed by load (ordinary answers of 0.5–1.2 s in `voice-commands-under-load`, whose scripted copy answers every confirmation after 1.2 s) is heard in full at the next confirmation, and its answers then raise the budget as before; speech stuck repeating (1.7–1.8 s idle, 2.1 s beside a heavy stage) is cut short again at 1.5 s, so a confirmation of it ends in about 1.55 s, under the 2 s bound with the margin the load takes away.

Alternatives: note nothing for a confirmation cut short (tried first in this change: the `under_load` component tests of `placed_commands`, whose every answer takes 1.2 s, and the deadline test of `confirm/tests/under_load.rs` lost every confirmation, since no answer ever came back to raise the budget); a token limit from the audio length (whisper-server takes no such field); move `e2e-resilience` to the heavy class (the budget would still raise itself on an idle machine and leave a 0.15 s margin).

**D2. An answer stuck repeating sets no pace.** An answer whose text is longer than 120 characters is speech recognition stuck repeating: a confirmation holds at most 2.5 s of speech, ordinary answers are 1–12 tokens (12–40 characters measured), while a loop is 219 tokens. Such an answer is still returned to the engine, which hears no command in it; only the pace ignores it. Without D2, a loop answered within a budget that slow ordinary answers had raised (1.6 s under a 1.9 s budget) would raise the budget to the deadline.

Alternatives: count words (a loop such as "E-e-e-e-…" is one word); time per token (verbose JSON without token counts per segment, and the cut-short case still has no answer).

**D3. The scenario prints its confirmation times when it passes.** The step "every confirmation of that speech is answered in under 2 seconds" writes the slowest time, the number of confirmations cut short and every time to the stage log, so each run under load measures the margin to the bound and the acceptance bar (three `full-check` runs of other changes) can be read from their logs.

## Risks / Trade-offs

- [A machine so loaded that every ordinary confirmation takes over 1.5 s never raises the budget past 1.5 s, and commands are cut short] → the old pace doubled the budget at each cut and reached the deadline after two; the ordinary answers measured beside a heavy stage stay under 0.7 s, the ones that stall for seconds were cut short before this change too, and any answer under 1.5 s raises the budget as before. The real-service tests of `placed_commands` raise their own floor (`fix-placed-commands-real-service-under-load`).
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
| Speech cut short does not lengthen the budget past 1.5 seconds | unit, `confirm/tests/under_load.rs` and `confirm/pace.rs` |
| An answer stuck repeating does not lengthen the budget | unit, `confirm/tests/under_load.rs` |

## Results

The branch's first `full-check` (2026-10-23, `e2e-resilience` from about 13:20, light class, beside the heavy model stages and builds of other sessions' checks, which held the machine lock in turn) passed the scenario with 60 confirmations of the Russian speech: 55 of them in 106–340 ms, 2 cut short (about 1.0 s), and the slowest 1.22, 1.41, 1.41 and 1.44 s, all within the 1.5 s budget a confirmation cut short gives; the bound of 2 s keeps a margin of 0.56 s, where the old pace left 0.15 s on an idle machine and none under load (the 2.116 s failure). The slow confirmations no longer reach 1.8 s, the time of a repetition loop decoded in full, so the budget no longer raises itself.

A separate run beside a load made by this change was dropped: the check's own run already ran beside other sessions' heavy stages, and a second run of `e2e-resilience` in this worktree would have shared `target/` with the running check. Every later run prints the same line, so the acceptance bar (three `full-check` runs of other changes) is read from their logs.

## Test limits

The branch's first `full-check` (2026-10-23, 13:00) passed every stage up to `e2e-bench`, which nextest stopped at its 6 min limit while it ran the accuracy sample of 100 recordings beside other sessions' light voice control stages. That limit dates from 159 s measured for the 100 recordings alone; the stage now also runs "Ordinary speech triggers no command" (20 recordings through the command spotting service), and its passes in the stage journal take 200–292 s, past half the limit, which `AGENTS.md` § Test limits does not allow. The limit is now 10 min, the smallest whole number of minutes at least twice the longest pass (292 s), in `.config/nextest.toml`.

## Out-of-scope failures

Two later `full-check` runs failed on tests this change does not touch: one recorded form of `voice_forms` (session-2 at 117.33 s, "that is all stop enter", its segment cut short at a 1074 ms budget that its ordinary answers had set, the same under the old pace) and six scenarios of `text_refinement.feature` (every dictation copy stalled 5.5–6.7 s at once). By the user's decision they are frozen centrally by `stop-filing-changes-for-flaky-tests` (branch `chore/freeze-flaky-tests`), not by this change, which waits for it and then merges `main`.
