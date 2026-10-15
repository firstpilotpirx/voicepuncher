# Design

## Context

See proposal.md (Why). This section covers only the current state that shapes the approach.

**How tests run today.** `_test` builds three selections with `cargo test --no-run` (app `--tests`, the command spotting crate, the E2E harness's `--lib`), starts all 34 test binaries at once from a bash loop, prints each binary's output when it ends, then the five slowest binaries and the failed targets. The E2E recipes call plain `cargo test`: the app's service binaries one after another (tests inside a binary in parallel), then `cucumber_e2e` with `--tags`. Nothing limits a test; `with-machine-lock` kills a run only after one hour.

**Measured baseline** (kit run logs of 2026-10-14/15 on this machine, Apple Silicon with 10 cores, current code):

| Run | Duration |
|---|---|
| `just test`, warm build | 63–81 s, median ≈ 69 s, not the documented ~20 s. The critical path is `commandspotting_reconnect_it::should_be_back_within_seconds_once_the_service_answers_after_a_long_outage`, which waits out a 60 s outage (61–65 s over ~40 runs). |
| `just check` | 374–410 s, median ≈ 398 s |
| `e2e-smoke` stage | 242–269 s: the app's service binaries take ≤ 34 s each (`voice_forms_it` is the longest) and ≈ 80 s together; the `@smoke` scenarios take ≈ 160–190 s. In one day the `@smoke` scenarios grew from ≈ 80 s to ≈ 190 s as scenarios were added. |
| `e2e-quick` | 26–62 s |
| `refine-eval` | 16–33 s |
| `e2e-bench` | 107–159 s |
| `e2e-resilience` | 150–159 s |
| `e2e-seams` | `chunked_transcription_it` (one test) 64–72 s, and 112–135 s on the code of 2026-10-11..14; the `@seams` scenarios take ≈ 30 s |

**Measured cost of one process per test** (existing debug build, under the machine lock): loading the command spotting models in a fresh process costs 0.37–0.43 s; that is one `engine_it` test alone against two tests in one process, over three rounds. Starting the app's lib unit test binary for one trivial test costs ≈ 7 ms (20 processes in 0.14 s).

**Waits that have no bound**, checked against the code:

- `SpottingClient::events_for_sent_audio` (`app/src/adapters/secondary/commandspotting/websocket/client.rs`) waits for the answer to its sync for as long as the service answers pings. This is the 2026-10-15 hang. Only tests call it: `commandspotting_client_it`, `voice_samples_it`, and `e2e-test/src/voice_commands.rs`, which in turn serves `commandspotting_websocket_it`, `voice_forms_it` and the E2E voice steps. The app's own path (`AudioStream`) uses `read_events` and `keep_alive` and never calls it.
- `commandspotting_websocket_it::should_be_refused_a_phrase_the_service_does_not_know` reads with no read timeout until the service closes the connection.
- `commandspotting_websocket_it::recognized_from_microphone` waits until 2 s pass without an event, with no overall bound while events keep coming.
- `command-spotting/tests/server_it.rs`: its `connect` makes the WebSocket handshake with no timeout.

**Already bounded**, so they need no change:

- `commandspotting_reconnect_it` uses deadlines (`feed_until`, `next_speech_start`), `recv_timeout` and the probe's 2 s connect wait.
- `http_failure_it` gives every request a 300 ms `ureq` timeout.
- `transcription_openai_it` and `textrefinement_openai_it` go through the adapters' request deadlines and use `recv_timeout(5 s)` for captured requests.
- The existing silent-service test of `commandspotting_client_it` relies on the client's 5 s silence limit.
- The component steps use `REACTION` deadlines.

Their stub threads may block forever, but they are not the test's thread.

**No tokio:** the workspace uses blocking `tungstenite`, blocking `ureq`, threads and `mpsc`. Cucumber steps run under `futures::executor::block_on`, and the E2E steps move blocking work off-thread.

**kit reads check output as libtest and cucumber text:** `test … FAILED`, `thread '…' panicked at <loc>:`, `test result: … N passed; M failed`, and the cucumber `✘` steps and `N scenarios (…)` (agent-kit `internal/failures`). It reads nothing of nextest.

**cargo-nextest is not installed.** `cargo install --list` shows nothing. Homebrew offers 0.9.148 (2026-10-07), which is under two weeks old and cannot be pinned. On crates.io, 0.9.146 (2026-09-21) is the newest release at least two weeks old. Building nextest needs Rust 1.91, while this repo pins 1.88. Running tests needs no newer toolchain (nextest's stability page).

## Goals / Non-Goals

**Goals:**

- One mechanism, nextest, bounds every Rust test, including each run of a Gherkin harness. Every limit lives in `.config/nextest.toml`.
- Every recipe selects the same tests as today, and the shared services carry the same load as today.
- A silent service fails a client wait inside the test, in seconds, naming the service and what the test waited for, well before the runner's limit.
- kit keeps naming failed tests with their location.

**Non-Goals:**

- Limits per scenario.
- Making `just test` faster. The 60 s outage test stays on its critical path.
- Teaching kit nextest's format (named in D9), or adding a machine-readable report such as JUnit.
- Anything the proposal puts out of scope.

## Decisions

### D1. cargo-nextest 0.9.146, required and pinned

- Pin 0.9.146: the newest release at least two weeks old.
- A private recipe `_require-nextest`, written like `_require-models`, checks that `cargo nextest --version` reports exactly 0.9.146. Otherwise it stops with `✗ cargo-nextest 0.9.146 not found (got '…') — run 'cargo +stable install --locked cargo-nextest@0.9.146'`.
- The guard is a dependency of `_test`, `_e2e-smoke`, `_e2e-quick`, `_e2e-seams`, `_e2e-bench`, `_e2e-resilience`, `_refine-eval` and `_test-real-keyboard`. `_release` gets it through `_test`.
- `.config/nextest.toml` also sets `nextest-version = { required = "0.9.146" }`, so a direct `cargo nextest run` with an older nextest refuses to run.
- `+stable` is needed because the repo's 1.88 toolchain cannot build nextest (MSRV 1.91). The installed binary then runs under the 1.88 cargo like any cargo subcommand.
- Rejected:
  - Homebrew cannot pin a version, and today it would install a 7-day-old release.
  - `curl get.nexte.st/0.9.146/mac | tar` is fast, but it pipes a download straight into `~/.cargo/bin`, while cargo installs a checksummed crate.
  - A minimum version alone is not exact pinning (exact-versions rule).

### D2. One config file: a default profile plus one profile per suite

`.config/nextest.toml`, a build input:

```toml
nextest-version = { required = "0.9.146" }

[profile.default]
slow-timeout = { period = "30s", terminate-after = 4 }   # SLOW at 30 s, stopped at 2 min (+10 s grace)
fail-fast = false              # every failure reported, as today's _test does
final-status-level = "slow"    # the run ends listing failed, timed-out and slow tests

[test-groups]
command-spotting-models = { max-threads = 1 }            # D7

[[profile.default.overrides]]
filter = 'binary(engine_it) | binary(placed_commands_it)'
test-group = "command-spotting-models"

[[profile.default.overrides]]   # D5: a 60 s outage by design
filter = 'binary(commandspotting_reconnect_it) & test(=should_be_back_within_seconds_once_the_service_answers_after_a_long_outage)'
slow-timeout = { period = "60s", terminate-after = 3 }

[profile.e2e-smoke]            # one profile per suite recipe: its scenario selection and its limits
success-output = "immediate"   # what `-- --nocapture` showed, per test
[[profile.e2e-smoke.overrides]]
filter = 'binary(cucumber_e2e)'
run-extra-args = ["--tags", "@smoke and not @wip"]
slow-timeout = { period = "60s", terminate-after = 7 }
# … e2e-quick, e2e-bench, e2e-seams, e2e-resilience, refine-eval, real-keyboard alike (D5)
```

- Suite profiles inherit the default profile and fall back on its overrides, so the model group and the reconnect override apply everywhere.
- Each suite recipe passes `--profile <recipe name>`. The real-keyboard profile is `real-keyboard`.
- Rejected:
  - A single E2E profile cannot give suites that run the same binary different limits.
  - Limits on the command line: nextest has no flag for them, and the limits would scatter over the justfile.

### D3. `_test` is one nextest run over the workspace, except the E2E harness

The recipe becomes `_test: _require-nextest`, then `cargo nextest run --workspace -E 'not binary_id(voicepuncher-e2e-test::cucumber_e2e)'`.

- That selects today's three selections plus D10's probe:
  - the workspace's default test targets are libs, bins and integration tests;
  - nextest, like the current loop, runs no doctests;
  - nextest does not even list a binary that a filter excludes.
- Parallelism: up to 10 tests at once over all binaries, where today every binary starts at once with its own threads. `just test` reaches no shared service, so this is only a CPU question. Measured costs (Context) put about 4 s of process starts across 10 slots, and D7's model chain beside the 61–65 s critical path. Wall time stays ≈ 69 s.
- The output summary comes from nextest:
  - PASS lines with each test's time;
  - SLOW lines;
  - the final list of failed, timed-out and slow tests, which replaces "slowest test binaries" and "N target(s) failed";
  - the `Summary [ … ] N tests run: …` line.

  The bash loop goes away.
- Cost: cargo now also builds `cucumber_e2e` in `just test`, because target selection cannot be set per package in one cargo call. That costs a few seconds, and only after an app or E2E-harness edit. `just check` builds it anyway.
- Rejected:
  - Three nextest runs, one per selection, run one after another (≈ 95 s) or print three interleaved summaries.
  - A `--test` glob list breaks silently when a test target is added.
  - Marking the harness `test = false` or giving it a required feature drops it from `cargo clippy --all-targets`.

### D4. The Gherkin harnesses speak nextest's custom-harness protocol as one test

`cucumber_integration` and `cucumber_e2e` each answer nextest's documented custom-harness protocol as a single unit, with one test named `scenarios`:

- `--list --format terse` prints `scenarios: test` and nothing else on stdout;
- `--list --format terse --ignored` prints nothing;
- `--exact scenarios --nocapture <extra…>` runs the scenarios with `<extra…>` as cucumber's command line, for example the suite profile's `run-extra-args = ["--tags", "@smoke and not @wip"]`;
- any other argument list is cucumber's own command line, as today, so `cargo test --test cucumber_e2e -- --name …` keeps working.

How the harnesses run:

- The argument reading lives in one module of the E2E harness library, which both harnesses already depend on, and is unit-tested there. `cucumber::cli::Opts` derives `clap::Parser` and `Cucumber::with_cli` exists in cucumber 0.23.
- A suite run is one nextest test, so nextest's limit, its process-group kill and its `TIMEOUT … voicepuncher-e2e-test::cucumber_e2e scenarios` line apply unchanged.
- Cucumber keeps its own concurrency: 4 scenarios at once, with `@accuracy` and `@serial` alone.
- The scenario selection sits next to the suite's limit in the suite profile, so a suite cannot run under another suite's limit, and a stray environment variable cannot narrow `just test`.

Rejected:

- Listing every scenario as its own test. That needs names for scenario-outline rows, test groups to recreate cucumber's concurrency and its run-alone tags, a process and a World per scenario (137 for `@smoke`), and limits per scenario name.
- A limit per scenario inside cucumber. Cucumber has no step timeout. A blocking step cannot be preempted on its single-threaded executor, and a watchdog thread could only exit the process, orphaning the scenario's own services.
- Running the harnesses outside nextest under `perl alarm` or `kit run --timeout`. That is a second limit mechanism outside `.config/nextest.toml`, and alarm kills `cargo` rather than the harness's process group.
- Passing the tags in an environment variable. It works, but it splits a suite's definition across two files, and a leftover variable in a shell would silently narrow `just test`.

### D5. Limits

The rule: a limit is the smallest whole number of minutes that is at least twice the longest run of the current code, and never below the 2 min default. Suite limits use 60 s periods, so the SLOW lines count minutes.

| Recipe | One nextest test is | Longest run | Limit |
|---|---|---|---|
| `test` | each test | ≤ 28 s, apart from the 60 s outage test (65 s) | default 2 min; the outage test 3 min |
| `e2e-smoke` | each test of the service binaries | 34 s (`voice_forms_it`) | default 2 min |
| `e2e-smoke` | the `@smoke` run | ≈ 190 s | 7 min |
| `e2e-quick` | the `@accuracy` run (10 samples) | 62 s | 3 min |
| `refine-eval` | the `@refinement` run | 33 s | default 2 min |
| `e2e-bench` | the `@accuracy` run (100 samples) | 159 s | 6 min |
| `e2e-resilience` | the `@resilience` run | 159 s | 6 min |
| `e2e-seams` | `chunked_transcription_it` | 72 s | 3 min |
| `e2e-seams` | the `@seams` run | ≈ 30 s | default 2 min |
| `test-real-keyboard` | the `@real_keyboard` run (2 scenarios, manual) | not measured | default 2 min |

- The grace period stays at nextest's default of 10 s: SIGTERM to the test's process group, then SIGKILL.
- A suite that grows past half its limit gets a new limit, twice its new longest run, in the same change. AGENTS.md says so (D11).
- Rejected:
  - One generous E2E limit lets `e2e-quick` hang for as long as `e2e-smoke` may legitimately run.
  - Margins under 2× would sit inside the spread already observed (`e2e-quick` ranges 26–62 s).

### D6. E2E recipes: one nextest run per test binary, in today's order

- `_e2e-smoke` runs, one after another:
  - each of `transcription_openai_it`, `textrefinement_openai_it`, `commandspotting_websocket_it`, `voice_samples_it` and `voice_forms_it`;
  - then `placed_commands_it`;
  - then `cucumber_e2e`.
- `_e2e-seams` runs `chunked_transcription_it`, then `cucumber_e2e`.
- The cucumber-only suites run `cucumber_e2e` once.
- Inside a binary, nextest's default of 10 test threads equals libtest's default, so the services see exactly today's load and order.
- `set -e` stops at the first failing binary, as `cargo test` does.
- Output:
  - `cucumber_e2e` runs with `--no-capture` and streams live as today. It is a single test, so serial running costs nothing, and the limits apply whatever the capture mode (nextest-runner arms its timers independently of it).
  - The app's binaries stay captured, with the suite profile's `success-output = "immediate"`. Their output, for example the seam sweep table, appears whole per test.
- Cost: ≈ 1 s of cargo and nextest start per extra run, ≈ +4 s on `e2e-smoke`.
- Rejected:
  - One nextest run over all of `e2e-smoke`'s binaries mixes, for example, 10 voice-control tests with the concurrent sessions of `voice_forms_it` and `voice_samples_it`. The command spotting service and the speech recognition copies its confirmations go to would see more load than today. Its confirmations give up after 3 s, and a test that needs speech recognition to reject a phrase would then flake.
  - `--no-capture` for every binary would serialize `commandspotting_websocket_it`'s 13 tests (+ ≈ 1 min).

### D7. Tests that load the command spotting models run one at a time

- `engine_it` and `placed_commands_it` share one model set per binary behind a mutex today. Their comment says ONNX Runtime aborts when models load or run on several threads.
- Under nextest every test loads its own models (0.4 s, measured). The `command-spotting-models` test group (max 1 thread) keeps them one at a time, as the mutex did. More would only add CPU load on 2-thread ONNX sessions, and in `e2e-smoke` it would add concurrent confirmations on the shared speech recognition service (D6).
- Cost:
  - `engine_it` ≈ 19–27 s + 19 × 0.4 s, run beside `just test`'s 61–65 s critical path, so `just test`'s time does not change;
  - `placed_commands_it` + 7 × 0.4 s ≈ +3 s on `e2e-smoke`.
- `server_it` loads its own models in its single test, as today, and stays outside the group.
- Rejected:
  - Unbounded parallel model tests change the load the tests were written for.
  - Merging the model tests into one test per binary would rewrite tests to suit a runner.

### D8. Client waits get a deadline, not `tokio::time::timeout`

`tokio::time::timeout` cancels a future at its next `.await`; it cannot stop a thread blocked in `read()`. Adding an async runtime around blocking clients would add a dependency and still need the blocking bound inside. The blocking equivalents are:

- a deadline the waiting loop checks (`Instant` plus `checked_duration_since`);
- a socket read timeout, so a blocking read returns control;
- `recv_timeout` on channels.

The repo already uses this pattern (the component steps' `REACTION`, `feed_until`). The sync wait already polls every 10 ms (`READ_WAIT`), so a deadline check costs nothing.

| Wait | Where the bound lives | Bound | Failure says |
|---|---|---|---|
| Answer to a sync, `events_for_sent_audio` | adapter method; the caller passes the bound (`within: Duration`) | real service: 20 s (`SYNC_ANSWER_LIMIT` in `e2e-test` `voice_commands`, also used by `voice_samples_it`). Stubs in `commandspotting_client_it`: 20 s for the service 12 s late, 2 s for the service that never answers the sync | the command spotting service did not answer sync N, the events for the audio sent, within the bound, though it answers pings |
| Silence, no answer at all | adapter, unchanged | 5 s silence limit | "command spotting service stopped answering" |
| Refusal of an unknown phrase | `commandspotting_websocket_it`: socket read timeout plus deadline | 5 s | the service did not refuse the unknown phrase within 5 s |
| Events of one utterance from the microphone | `commandspotting_websocket_it`: overall deadline over the `recv_timeout(QUIET)` loop | 30 s | the service still sent events 30 s after the speech |
| WebSocket handshake with the in-process service | `server_it`: connect timeout plus socket timeouts during the handshake | 5 s | the service did not complete the handshake within 5 s |
| HTTP requests | adapters and `ureq`, unchanged | per request 5 s + half the audio (≤ 60 s), 3 attempts; refinement 30 s; failure stub 300 ms | "speech recognition request to … failed"; WARN "speech recognition service did not answer in time" |

**Values.** 20 s is more than 5 times the service's worst normal answer: its confirmations give up after 3 s (`--confirmation-deadline-secs`), plus processing 2 s of audio. 5 s is more than 10 times an answer that comes at once. 30 s is 6 times an utterance plus 2 s of quiet. Every bound is far below the runner's limits, so the test, not nextest, reports a silent service.

**The adapter, the only app code touched.** `events_for_sent_audio` gains the bound because the wait loop lives there, and the error belongs where the protocol is known.

- The app's path is unchanged: `AudioStream` never calls this method.
- `commandspotting_client_it::should_wait_for_a_service_that_answers_pings_however_late_its_events_come` keeps its meaning with a 20 s bound around 12 s-late events. Its name loses "however late".
- Rejected:
  - Bounding the call from outside with a thread plus `recv_timeout` leaves a thread pinging the shared service until the harness exits.
  - Moving the sync wait into `e2e-test` would duplicate the protocol parsing.

### D9. kit's readers: kept working from this repo where possible; one agent-kit change named

From this repo:

- The justfile exports `NEXTEST_NO_OUTPUT_INDENT=1`, the environment form of `--no-output-indent`. libtest's own `test x ... FAILED` and `thread 'x' panicked at <file:line:col>:` lines then start at column 0 inside nextest's failure output, so `kit run` keeps listing failed tests with location and message.
- Cucumber output is unchanged.
- `final-status-level = "slow"` puts the TIMEOUT, FAIL and SLOW tests just above nextest's closing `error: test run failed`, inside the 30-line tail that kit prints when it parses no failure.

What stays wrong until agent-kit learns nextest:

- A TIMEOUT next to a parsed failure is missing from kit's failure list.
- kit's "N passed, M failed" counts only the tests whose output nextest printed. That gives none on a green `just test` and per-test counts in the E2E profiles.
- The test-results card finds no results on a green nextest run.
- `kit log --test` does not find nextest's output blocks.

The agent-kit change **`read-nextest-output`** fixes this. It is filed in agent-kit with `create-change` and is out of scope here. It should:

- read nextest's status lines (`FAIL`, `TIMEOUT`, `SIGSEGV`/`SIGABRT`, `LEAK-FAIL`) as failures named `<binary-id> <test>`;
- read its `Summary [ … ] N tests run: X passed, Y failed, Z timed out` line for the verdict summary and the test-results card;
- read its output blocks for `kit log --test`.

nextest's human-readable output is outside its stability policy, so that change keys on the format of the pinned version.

Rejected: printing synthetic libtest lines (`test result: …`) from the recipe. That fakes another tool's format, and kit would double-count the failed tests' own lines.

### D10. Hang probes stay in the repo, opt-in

`VOICEPUNCHER_HANG_PROBE=1` arms two probes; without it they cost nothing:

- **Test probe.** A test target in the E2E harness crate, `e2e-test/tests/hang_probe.rs`, which `just test` runs (D3).
  - Unset: it prints the usual `GATED:` line and passes at once.
  - Set: it sleeps forever.
- **Scenario probe.** A `before` hook in `cucumber_e2e`'s `main`.
  - Set: it prints the scenario it holds and blocks it forever.
  - It is not a scenario, so no `.feature` holds a scenario without a spec.

The proof: `VOICEPUNCHER_HANG_PROBE=1 just test` and `VOICEPUNCHER_HANG_PROBE=1 just e2e-quick` (3 min limit).

**Why keep them:**

- A nextest bump (exact pin, so always deliberate) or a config edit can break the mechanism silently; re-checking it costs one command.
- The `GATED` test adds a trivial test to `just test` and no time.

Rejected: a one-off test removed before merge. It proves the mechanism once and leaves no way to check it again.

### D11. Documents and configuration

- **README.md**
  - § Install prerequisites: add cargo-nextest 0.9.146 with the install command.
  - § Testing:
    - tests run under nextest with a limit per test (2 min by default, each E2E suite its own, `.config/nextest.toml`);
    - a test over its limit is reported `TIMEOUT`;
    - the probes;
    - the measured times.
- **AGENTS.md § How to verify a change**
  - nextest is required (the guard installs nothing);
  - `TIMEOUT` means hung: report it, never rerun it unchanged, never raise a limit to let a hang pass;
  - the growth rule of D5;
  - `.config/nextest.toml` joins the build inputs;
  - the times become the medians measured after the change: ≈ 70 s and ≈ 7 min expected, replacing the stale ~20 s.
- **openspec/config.yaml** `context`: the same times.
- **`.claude/kit.yaml`** `checks.build_inputs`: add `".config/**"`.

### D12. Manual run: none

The only app file touched is `client.rs`, and only the test-only `events_for_sent_audio` changes. `kit checks-needed` flags "voice control" by path alone. The PR body says why stage 10 does not apply.

## Test levels

There are no specs (`skip_specs`). Each acceptance item of proposal.md, and each new behavior, is proven at the level below.

| What is proven | Level | How |
|---|---|---|
| The harness argument reading: a listing is one test, `--ignored` lists none, a nextest run keeps only the extra arguments, a direct run keeps cucumber's own | unit (E2E harness library) | Red, then Green; run by `just test` |
| The sync wait ends within its bound, naming the service and the sync, when the service answers pings but never the sync | adapter integration (`commandspotting_client_it`) | new test, Red first: it hangs and nextest reports TIMEOUT at 2 min, so nextest must come first; Green: the bound |
| The sync wait still waits for late events inside its bound | adapter integration (existing test, bound passed) | `just test` |
| A WebSocket service that accepts and stays silent fails the wait in seconds, naming the service | adapter integration (existing `should_fail_the_wait_when_the_service_stops_answering`) | unchanged, `just test` |
| An HTTP service that accepts and stays silent fails the request in seconds, naming the service | adapter integration (existing `should_give_up_a_short_recording_the_service_never_answers_within_half_a_minute`) | unchanged, `just test` |
| The waits bounded inside test code (refusal, microphone loop, handshake) | none: test code | review. A test of a test is not written. |
| A hung test in `just test` is stopped at 2 min + ≤ 10 s, named TIMEOUT, the recipe exits non-zero, and the lock frees at once | recipe run by the agent at stage 6 (not a manual run of the app) | `VOICEPUNCHER_HANG_PROBE=1 just test`, with a second `just test` queued behind it. It must take the lock within a second of the first one's exit. The excerpt and timings go in the PR. |
| A hung scenario in an E2E recipe is stopped at its suite limit, the harness and the scenario are named, the recipe exits non-zero, and the services stop | recipe run (stage 6) | `VOICEPUNCHER_HANG_PROBE=1 just e2e-quick` |
| `test`, `check` and `full-check` pass and run as many tests as before; the slow suites pass under their limits | recipe runs (stages 5 and 8) | `cargo nextest list` against `cargo test -- --list` for the libtest tests; cucumber's `N scenarios` per suite against the baseline logs |
| Recipe names and arguments are unchanged, and every check runs under `with-machine-lock` | review | `just --list` before and after; every public recipe is still `{{locked}} _<name>` |
| `just test` and `just check` keep their times | recipe runs (stage 5) | median of the runs during apply ≤ 1.1 × baseline median (`just test` 69 s, `just check` 398 s) |

## Risks / Trade-offs

- [A suite outgrows its limit as scenarios are added] → limits are at least 2× the longest run, SLOW lines count minutes, and AGENTS.md says to raise a limit in the change that adds the scenarios. A TIMEOUT from growth fails loudly; it does not hang.
- [A timed-out harness names no scenario in nextest's line] → the cucumber run streams (`--no-capture`), so the log ends with the scenarios that were running, and the scenario probe prints its scenario. Limits per scenario were rejected (D4).
- [nextest's human output can change between releases] → exact pin; a bump re-runs both probes, and the agent-kit change keys on the pinned format.
- [kit counts and the test-results card are wrong until `read-nextest-output` lands] → failure names and locations keep working, and TIMEOUT shows in the final status lines. The degradation is named, not hidden.
- [Process per test] → measured at 7 ms per start and 0.4 s per model load; neither touches the critical paths (D3, D7).
- [`cucumber_e2e` is built in `just test`] → a few seconds, only after app or harness edits; it is measured in the time bar.
- [The harness glue misreads arguments if nextest changes its protocol] → the protocol is documented and the version pinned; unit tests cover it, and a broken listing fails the run loudly.
- [Stopping a harness also stops the services its scenarios started] → they share the test's process group, and their supervisors stop on SIGTERM. The shared services belong to the recipe and stop through its trap.
- [A machine or worktree without nextest] → the guard prints the one-time install. Branches that have not merged `main` keep `cargo test` and queue on the same lock.

## Migration Plan

1. Install once per machine at stage 5, before the first nextest recipe: `cargo +stable install --locked cargo-nextest@0.9.146`. It is a few minutes of build, and the guard prints the same command.
2. Order of work: first the nextest switch, its config and the harness protocol, then the client bounds, so that the sync bound's Red test shows a TIMEOUT. The docs come last, with the measured times.
3. Rollback: revert the squash commit. The recipes return to `cargo test`, and nextest can stay installed.
