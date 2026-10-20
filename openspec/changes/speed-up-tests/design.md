# Design

## Context

See proposal.md for why. The E2E harness already runs up to 4 scenarios at once; `@serial` scenarios stream audio in real time one at a time. The command spotting service's confirmation wait (3 s) and the transcriber's request timeout are already settable; the client's reconnect pauses and the supervisor's cadence are not.

## Goals / Non-Goals

**Goals:** every behavior still proved once; `just test` < 20 s; `e2e-smoke` in `check` at least halved.

**Non-Goals:** no production timing changes; `@serial` scenarios keep running alone.

## Decisions

1. **Client timings as an adapter parameter.** The command spotting client takes its reconnect pauses and replay window from a timings value whose default is today's constants; the app always uses the default, the reconnect test passes one scaled down tenfold. The test still proves the tenth attempt and the 5 s return, in about 6 s. Rejected: moving it out of `just test` (still 60 s of a check); an env var in the app (a production knob).
2. **Give-up test with a 1 s request timeout.** The wait is the shorter of the audio-length deadline and the configured timeout; the test sets 1 s, so three hung attempts take about 4 s. The growth with audio length stays proved by the adapter's unit tests. No app code changes.
3. **Supervisor cadence settable.** The justfile's health check reads its interval from an env var, default 5 s; the `@resilience` steps start their own services with 1 s, so "hangs" restarts within seconds and "busy" streams for 6 s across several checks instead of a minute.
4. **Stall of 11 s.** The confirmation stall stays just past the 10 s the scenario names; shorter would stop proving that bound.
5. **Smaller voice control set by tag.** `@smoke` keeps, per command, 2–3 synthesized scenarios plus every `@voice_samples` scenario; the rest move to a new `@regression` tag. `full-check` runs a new stage `e2e-regression` after `e2e-smoke`, under the same lock and services, so it still runs the whole set. Rejected: a larger `e2e-smoke` in `full-check` (the journal would mix two sizes).
6. **Concurrency measured, then fixed.** The harness reads its concurrency from an env var for the measurement (4, 6, 8, three green runs each); the fastest green value becomes the default.
7. **Refinement evaluation once.** The three evaluation scenarios share one report computed on first use in the harness process.
8. **Duplicates.** The Gherkin scenario stays where a spec scenario traces to it; the plain Rust test stays where it checks an adapter detail. The PR lists each removed test and the test that keeps its behavior.

## Test levels

| Spec scenario | Level |
|---|---|
| Commands work again after the command spotting service restarts | E2E `.feature` (`@resilience`) |
| Listening is not given up after many failed attempts | adapter integration |
| A confirmation that takes longer than 10 seconds keeps listening on | E2E `.feature` (`@resilience`) |
| A pause of the app's own does not lose the service | adapter integration |
| A burst of audio after long listening does not lose the service | adapter integration |
| A hung short chunk is given up quickly | adapter integration |
| The command spotting service hangs | E2E `.feature` (`@resilience`) |
| A busy service is not restarted | E2E `.feature` (`@resilience`) |
| One confirmation hangs | adapter integration (command spotting service) |
| Commands are spotted after a hung confirmation | adapter integration (command spotting service) |

## Risks / Trade-offs

- [Scaled timings hide a race] → the ratios stay the same; `@resilience` keeps real services.
- [A smaller `check` lets a voice control regression reach `full-check`] → `full-check` runs before every merge, so nothing merges untested.
- [Higher concurrency adds flakes] → a value is kept only after three green runs; `@serial` stays alone.

## Dependencies

All three declared dependencies (`show-spotting-away`, `speed-up-voice-start`, `fix-flaky-tests`) are archived on `main`.
