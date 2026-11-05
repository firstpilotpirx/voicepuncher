# Design

## Context

An utterance reports from both recognizers through one path (`Utterance::report`): a finding is dropped when it `doubles` the utterance's last report (the same commands within 0.6 s, or the end of a longer command word). The English recognizer stamps a word 0.65–1.1 s later than the Russian one (the service log of 2026-11-03/04), so its finding of the same command lies beyond the gap and goes out again; the gap cannot grow, as 0.6 s already separates two commands of one recognizer ("enter … enter" in a noisy room).

## Goals / Non-Goals

- Goals: one report per command said, whichever recognizers heard it and however they stamp it.
- Non-goals: different commands reported by the two recognizers of one chain (`settle-chain-tail-at-the-close`); the handover between utterances; the recognizers' stamps themselves.

## Decisions

**D1. Repeats are counted, not timed.** The utterance keeps, beside its last report, how many times each recognizer heard each set of commands (every finding `decide` returns for it, reported or not) and how many reports of that set went out. A finding is a repeat when the utterance already holds as many reports of its commands as the recognizer's hearings of them, the new one counted: both recognizers hear the same speech, so the n-th occurrence in one is the n-th in the other, whatever the stamps. The existing `doubles` stays and is checked too: it still drops a recognizer's own second word of a doubled form settled after its first ("старт", then "старт старт" within 0.6 s) and the "start" another recognizer heard of a "restart", neither of which the count sees. Kept out: a wider gap across recognizers (the English lag varies and overlaps the pause that makes two commands); comparing only the final texts at the close (the log shows the second report before the close too). The counts live in the utterance's report state (`Reports`, moved to its own `session/reports.rs` to keep `utterance.rs` within its size budget; `Reports::goes_out` holds both checks); `doubles` in `decide/mod.rs` stays pure and unchanged.

**D2. A false hearing stands in for a real one.** When one recognizer hears a command the other did not and the other later hears the same command once, the count takes the second for the first: one report instead of today's two. For "enter" that is the press asked for; after a false "stop" the dictation has ended already, and the real one would be ignored in standby.

## Test levels

| Spec scenario | Test |
|---|---|
| A command both recognizers hear is reported once | existing E2E `@smoke` scenario, unchanged; existing session test |
| A doubled command is reported once | existing E2E `@smoke` scenario, unchanged |
| A command the recognizers time a second apart is reported once | unit: session test with scripted recognizers in `command-spotting/src/streaming/session/tests/` (red first); the user's voice: `voice_samples` on sessions 1–6 and `just streaming-bench --recorded` |
| A command said twice with a pause between is reported twice by both recognizers | unit: session test with scripted recognizers (red first) |

The E2E rule keeps its heading and scenarios: the real recognizers' stamps cannot be scripted there.

## Risks / Trade-offs

- D2 drops a second real command only when one recognizer missed the first and heard the second alone: both recognizers hear every clear command of the user's recordings, and the voice-sample tests guard it.
- `settle-chain-tail-at-the-close` edits the same two files; the count rule touches only `Reports` and `report`, so a merge resolves at that function.
