# Research: where `check` and `full-check` spend the machine lock

Measured on 2026-10-14/15 from the `kit run` logs of every `just check` and `just full-check` run on this machine (`$TMPDIR/run-visible/*.log`): the stage table that `_stages` prints at the end, the log's start time (file name) and end time (modification time). Several voicepuncher sessions ran changes in parallel worktrees on both days.

## Stage times today

`just check` — 24 runs, 14 green (median, min–max, seconds):

| Stage | Median | Range | Needs the model services, GPU or shared ports |
|---|---|---|---|
| fmt | 0.3 | 0.3–0.5 | no |
| lint | 4 | 0.3–37 (≈ 27–37 after an edit) | no |
| test | 66 | 16–81 | no |
| build | 5 | 0.2–50 (≈ 45 after an edit) | no |
| e2e-smoke | 245 | 183–269 | yes |
| e2e-quick (WER on 10 recordings) | 30 | 27–62 | yes |
| refine-eval | 23 | 16–33 | yes |
| **total, green runs** | **371** | 282–410 | |

`just full-check` — 24 runs (13 with `cargo clean` and every stage up to `e2e-seams`, which failed on the known WordPauses bug of `fix-dropped-phrase-at-word-pauses`; 11 reruns of the later stages on their own):

| Stage | Median | Range | Needs the model services, GPU or shared ports |
|---|---|---|---|
| `cargo clean` | ≈ 20–60 | (not in the stage table: time outside the stages of runs that did not wait) | no |
| fmt | 0.4 | 0.3–0.5 | no |
| lint (cold) | 28 | 25–33 | no |
| test (cold) | 120 | 65–127 | no |
| build (cold) | 45 | 39–55 | no |
| e2e-smoke | 248 | 164–257 | yes |
| e2e-seams | ≈ 140 | `chunked_transcription_it` 65–104 until it failed, `@seams` scenarios ≈ 53 (speed-up-checks research) | yes |
| e2e-resilience | 151 | 150–151 | yes (own ports, but GPU and CPU) |
| e2e-bench (WER on 100 recordings) | 130 | 108–145 | yes |
| refine-eval | 27 | 17–32 | yes |
| **total, estimated** | **≈ 920** | | |

## What the lock covers today, and what it would cover

| Recipe | Lock held today | Stages that need no lock | Lock held when only the model stages lock |
|---|---|---|---|
| `check`, warm (no edit since the last build) | ≈ 375 s | fmt + lint + test + build ≈ 75 s (20 %) | ≈ 300 s |
| `check`, after an edit | ≈ 465 s | ≈ 165 s (35 %) | ≈ 300 s |
| `full-check` | ≈ 920 s | `cargo clean` + cold fmt/lint/test/build ≈ 225 s (25 %) | ≈ 695 s |
| `full-check`, WER on 10 recordings instead of 100 | | + e2e-bench − e2e-quick ≈ 100 s | ≈ 595 s (35 % less than today) |

The E2E recipes also compile inside the locked stage what the stages before them did not build: `cucumber_e2e` (the `test` stage already builds the `*_it` binaries of `e2e-smoke` and `placed_commands_it`; design.md § Context) — nothing warm, 2.9 s after an edit of `e2e-test/src/` (the Red run of task 2.1), 5–6 s on a fresh `target/` in the two `full-check` logs of 2026-10-15 15:38 — and the command spotting service's own release build when a run starts that service on a fresh `target/`. After `e2e-resilience`, the next model stage's `cargo build --release` also compiled ring, rustls, ureq, sherpa-onnx and the command spotting service again (7.07 and 7.57 s in `_stages` runs of 2026-10-14 21:48 and 2026-10-15 15:27): the `@resilience` steps run `just start-model-servers` inside `cargo test`, whose variables ring's build script tracks (design.md § Context; found and fixed at verify, see 7.2).

## How long runs waited

The wait is a run's wall time minus the sum of its stage times (for `full-check` this includes `cargo clean`, ≈ 20–60 s). On 2026-10-15 until 15:54:

| Recipe | Runs | Waited in total | Median wait | Longest wait | Waits over 5 min |
|---|---|---|---|---|---|
| `check` | 19 | 94 min | 75 s | 27 min | 7 |
| `full-check` | 8 | 52 min | 6.8 min | 21 min | 4 |

Together the runs held the lock for ≈ 145 min and waited for it ≈ 147 min. Every second a run holds the lock without needing it is a second every queued run waits.

## Measured on this branch (2026-10-16)

Commit `0457442` (groups 1–6), worktree of the change, services already running (no run started or stopped them), `VOICEPUNCHER_BENCH_CORPUS` at the main checkout's corpus. Every run through `kit run`; the numbers are the rows of the stage table `_stages` prints. No other session took the machine lock during these runs: every `lock wait` is ≤ 0.4 s except the second run of the parallel pair (7.3), which waited for the first run of the pair.

**Recordings of the user's voice.** `voice_forms_it` fails on the user's session-2 recording at 27.23 s ("restart"; owned by `recognize-users-quiet-restart`, same on `origin/main`). That failure ends `e2e-smoke` before it has run in full: cargo stops at the first failing test binary, so `voice_samples_it`, `placed_commands_it` and all `@smoke` scenarios (≈ 200 s of the stage) did not run, and the lock hold read 69 s instead of ≈ 270 s (two discarded runs). The runs below therefore set `VOICEPUNCHER_VOICE_SAMPLES` to a copy of `~/.voicepuncher/voice-samples` without that one line of `session-2/forms.jsonl` (81 of 82 recorded forms), so `e2e-smoke` runs to its end. Where a run still failed, its lock hold is the `lock held` row of the failed run plus the row of the stages after the failing one run on their own (`just _stages <the rest>`), added up.

### Lock hold of `check` (7.1) — bar ≤ 345 s

`just check`, alone, three runs after an edit (`touch app/src/lib.rs e2e-test/src/lib.rs command-spotting/src/lib.rs`) and three warm runs, interleaved:

| Run | fmt | lint | test | build | build-e2e | e2e-smoke | e2e-quick | refine-eval | lock held | total |
|---|---|---|---|---|---|---|---|---|---|---|
| edit 1 | 0.4 | 2.9 | 74.9 | 4.7 | 4.4 | 271.6 | 28.9 | 20.1 | 321.4 | 409.1 |
| edit 2 ¹ | 0.4 | 2.8 | 74.7 | 4.6 | 4.3 | 270.5 ✗ | 28.7 | 19.6 | 271.2 + 48.9 = 320.1 | 358.5 + 49.2 |
| edit 3 | 0.4 | 3.0 | 74.8 | 4.8 | 4.4 | 267.0 | 27.8 | 18.4 | 313.8 | 401.6 |
| warm 1 | 0.3 | 0.4 | 65.9 | 0.2 | 0.9 | 269.4 | 28.7 | 18.2 | 316.8 | 385.2 |
| warm 2 | 0.3 | 0.4 | 65.8 | 0.4 | 0.9 | 269.4 | 29.1 | 18.1 | 317.2 | 385.6 |
| warm 3 | 0.4 | 0.4 | 65.8 | 0.2 | 0.9 | 266.8 | 27.8 | 18.6 | 313.8 | 382.1 |

¹ The last `@smoke` scenario, "Requests that arrive at once are shared by the copies" (`local_services.feature`), failed once ("each of the three copies transcribed some of them": one HAProxy copy got no request) after every other scenario ran, with no other run beside it, and passed in every other run: a flake, to be named in the PR; `e2e-quick refine-eval` then ran on their own.

- **Median lock hold: 320 s after an edit, 317 s warm — bar ≤ 345 s met** (expected ≈ 300 s; today's whole-run lock held ≈ 375–465 s).
- Unlocked stages against the research medians (fmt 0.3, lint 4, test 66, build 5): lint 0.4 warm / 2.9 after an edit and build 0.2–0.4 / 4.7, within; fmt 0.3–0.4, up to 0.1 s over its median, within its range 0.3–0.5. Timed alone at verify (2026-10-16 03:51, under the machine lock), run as `_stages` runs it — `MACHINE_LOCK_DIR=$(mktemp -d) ~/.claude/bin/with-machine-lock just _fmt`, with and without `taskpolicy -c utility` in front, alternating — fmt took 0.32–0.33 s unclamped and 0.32–0.35 s clamped in 10 pairs (medians 0.32 and 0.325 s): the clamp costs it at most 0.03 s, so the 0.4 s readings are not the clamp's. `test` takes 65.8 s warm, within, but 74.8 s after an edit, 13 % over the 66 s median (the research median mixes warm and edited runs, range 16–81 s). Counted apart, the 13 `check` runs of 2026-10-15 that recompiled (lint or build over 1 s) took 65.6–81.1 s in `test`, median 70.1 s, and the 2 warm ones 65.7 and 66.0 s: 74.8 s is within that range, 7 % over its median, with an edit that touches all three crates. The unlocked stages together take 68 s warm and 87 s after an edit, against ≈ 75 s and ≈ 165 s in the research.
- The clamp alone (measured at verify, 2026-10-16 03:25, under the machine lock, no other build running): after the same edit, `_test`'s three `cargo test … --no-run` builds took 7.7–8.7 s unclamped (median 8.3 s) and 8.3–9.5 s under `taskpolicy -c utility` (median 8.85 s), in four alternating pairs after a warm-up, the clamped one slower in each pair: ≈ 0.6 s, 7 % of the build and under 1 % of the `test` stage. The other ≈ 8 s that `test` gains after this edit are the rebuild of all three crates. To repeat: `touch app/src/lib.rs e2e-test/src/lib.rs command-spotting/src/lib.rs`, then time `cargo test -p voicepuncher --tests --no-run`, `cargo test -p voicepuncher-command-spotting --no-run` and `cargo test -p voicepuncher-e2e-test --lib --no-run`, with and without `taskpolicy -c utility` in front, alternating.
- Wall time: ≈ 385 s warm, ≈ 405 s after an edit (≈ 6.5 min).

### Lock hold of `full-check` (7.2) — bars ≤ 685 s (10 recordings), ≤ 800 s (100)

Both runs failed at `e2e-seams` after it ran in full: 7 of the 29 `@seams` scenarios "Long speech under difficult recording conditions" (10–19 % missing words in Russian, a misheard "with the" in English), owned by `raise-accuracy-under-hard-conditions`. The stages after it ran on their own; a first rerun that began with `e2e-seams` failed there the same way (91.9 and 92.8 s), so the failure is steady, not a flake, and the sums leave that rerun out.

| Sample | Command | Rows | Lock held |
|---|---|---|---|
| 10 | `just _stages fmt lint test build build-e2e e2e-smoke e2e-seams e2e-resilience e2e-quick refine-eval`, then `just _stages e2e-resilience e2e-quick refine-eval` | unlocked 67.7; e2e-smoke 265.2, e2e-seams 92.3 ✗ (held 358.1); e2e-resilience 168.0, e2e-quick 34.2, refine-eval 17.8 (held 220.6) | **578.7 s — bar ≤ 685 s met** (expected ≈ 595 s) |
| 100 | `just full-check` (prints `● accuracy: 100 recordings (e2e-bench) — recognition paths changed: justfile`), then `just _stages e2e-resilience e2e-bench refine-eval` | unlocked 67.7; e2e-smoke 265.2, e2e-seams 92.7 ✗ (held 358.4); e2e-resilience 167.0, e2e-bench 102.1, refine-eval 18.9 (held 288.6) | **647.0 s — bar ≤ 800 s met** (expected ≈ 695 s) |

Wall time, added up the same way: ≈ 650 s (≈ 11 min) with 10 recordings, ≈ 715 s (≈ 12 min) with 100; today's `full-check` held the lock ≈ 920 s.

Both runs of the stages after `e2e-seams` compiled under the lock: the stage after `e2e-resilience` began with ring, rustls, ureq, sherpa-onnx and the command spotting service built again (6.46 and 6.69 s, inside the `lock held` above), because the `@resilience` steps had built the service inside `cargo test`, whose variables ring's build script tracks (design.md § Context). Found at verify and fixed there: `start-model-servers` now builds without those variables. With the fix, `just _stages build build-e2e e2e-resilience e2e-quick` (2026-10-16 05:12) shows no `Compiling` line after `● … machine lock taken` — `e2e-quick`'s `cargo build --release` takes 0.16 s — with `e2e-resilience` 161.1 s, `e2e-quick` 28.1 s and the lock held 190.0 s, all passing.

### Two checks at once (7.3)

Two worktrees (this one and a scratch worktree on the same commit), each after an edit, `just check` started together at 00:59:36:

| Run | fmt | lint | test | build | build-e2e | `→ waiting` | `● taken` | lock wait | lock held | total |
|---|---|---|---|---|---|---|---|---|---|---|
| A | 0.3 | 3.4 | 83.3 | 4.8 | 7.5 | 01:01:16 | 01:06:25 | 308.6 | 314.4 | 722.4 |
| B | 0.3 | 3.3 | 76.1 | 4.8 | 9.4 | 01:01:10 | 01:01:11 | 0.3 | 313.5 | 407.8 |

Both ran fmt, lint, test, build and build-e2e at the same time (00:59:36 to ≈ 01:01:15) and printed `→ waiting` only after `build-e2e`; A waited only for B's model stages. Both passed. Run together, their unlocked stages took 99 s and 94 s against 87 s alone.

### CPU clamp (7.4) — bar: 3 of 3 pass, model stages ≤ 15 % slower

`just check` (warm) in this worktree; when its log showed `● … machine lock taken`, the scratch worktree ran `cargo clean`, then `just _stages fmt lint test build build-e2e` (≈ 230 s: lint 29–32, test 137, build 51–53, build-e2e 10.5), which overlapped `e2e-smoke` (the build ended ≈ 40 s before `e2e-smoke` did):

| Run | e2e-smoke | e2e-quick | refine-eval | model stages | lock held | result |
|---|---|---|---|---|---|---|
| overlap 1 | 268.9 | 27.8 | 20.1 | 316.8 | 317.5 | ✓ |
| overlap 2 | 269.2 | 27.6 | 17.7 | 314.5 | 315.2 | ✓ |
| overlap 3 | 268.4 | 27.8 | 19.2 | 315.4 | 316.3 | ✓ |
| alone (the six 7.1 runs, same day) | 266.8–271.6 | 27.8–29.1 | 18.1–20.1 | 313.2–320.6, median 316.5 | | |

**All three passed; model stages 315.4 s median against 316.5 s alone — no slowdown (bar ≤ 15 %, ≤ 364 s), met.** Against the three warm runs alone, as planned (the overlap runs were warm too): 313.2–316.6 s, median 316.3 s, the same verdict.

## Sharing the model services between two runs (8, design decision 7)

Measured on 2026-10-16, 01:52–02:43, on commit `8bb045c` (groups 1–7). Two scratch worktrees A and B, detached at that commit, prebuilt with `just _stages build build-e2e`. The services already ran (started by `start-model-servers` the evening before, not by any run; no `*.e2e` marker in `~/.voicepuncher/run/`), so the script neither started nor stopped one. The user was not asked to stay idle: a long-lived connection to the command spotting service (its `conn=3` stats lines) stayed open the whole time, so the user's own app may have used the services meanwhile.

**A run** is `just _e2e-smoke`, `just _e2e-quick` and `just _refine-eval` in one worktree with `VOICEPUNCHER_E2E_SERVICES_HELD=1`, `VOICEPUNCHER_BENCH_CORPUS` at the main checkout's corpus and `VOICEPUNCHER_VOICE_SAMPLES` at the copy without session-2's 27.23 s "restart" (above), every stage run even after a failing one. **A pair** is A then B (S) or A and B started together (C), inside one `~/.claude/bin/with-machine-lock` call; eight pairs S C S C S C C C. Restarts are the `exited with code` and `stopped answering its health check` lines of the shared services' logs (`~/.voicepuncher/logs/{whisper-server-1808[234],haproxy-8081,llama-server-8080,voicepuncher-command-spotting-8082}.log`), counted before and after each pair; the supervisors' PIDs were compared too.

| Pair | Wall | Restarts | Run | e2e-smoke | e2e-quick | refine-eval | Run time | Mean WER | No command | Failed |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 S | 631.9 | 0 | A | 269.5 | 27.9 | 19.1 | 316.5 | 0.088 | ✓ | — |
| | | | B | 268.6 | 27.8 | 18.7 | 315.1 | 0.088 | ✓ | — |
| 2 C | 98.6 | 0 | A | 21.6 ✗ | 40.4 | 36.4 | 98.6 | 0.088 | ✓ | phrase set switched ² |
| | | | B | 21.6 ✗ | 40.4 | 36.4 | 98.5 | 0.088 | ✓ | phrase set switched, inside a sentence ² |
| 3 S | 625.9 | 0 | A | 266.0 | 28.0 | 19.5 | 313.6 | 0.088 | ✓ | — |
| | | | B | 265.8 | 27.8 | 18.6 | 312.2 | 0.088 | ✓ | — |
| 4 C | 316.8 | 0 | A | 270.9 | 28.0 | 17.8 | 316.7 | 0.088 | ✓ | — |
| | | | B | 21.4 ✗ | 28.7 | 25.7 | 76.0 | 0.088 | ✓ | inside a sentence ² |
| 5 S | 626.4 | 0 | A | 266.2 | 27.7 | 19.5 | 313.4 | 0.088 | ✓ | — |
| | | | B | 265.1 | 28.1 | 19.7 | 313.0 | 0.088 | ✓ | — |
| 6 C | 315.7 | 0 | A | 270.0 | 27.8 | 17.9 | 315.7 | 0.088 | ✓ | — |
| | | | B | 21.4 ✗ | 28.7 | 25.4 | 75.5 | 0.088 | ✓ | phrase set switched, inside a sentence ² |
| 7 C | 99.2 | 0 | A | 21.4 ✗ | 40.5 | 37.0 | 99.0 | 0.088 | ✓ | phrase set switched, inside a sentence ² |
| | | | B | 21.4 ✗ | 40.9 | 36.8 | 99.1 | 0.088 | ✓ | phrase set switched, inside a sentence ² |
| 8 C | 314.3 | 0 | A | 268.4 | 27.7 | 18.1 | 314.2 | 0.088 | ✓ | — |
| | | | B | 21.3 ✗ | 29.1 | 25.6 | 76.0 | 0.088 | ✓ | inside a sentence ² |

² `commandspotting_websocket_it`: `should_recognize_commands_after_long_listening_with_the_phrase_set_switched_between_them` and `should_ignore_a_command_phrase_said_inside_a_sentence_after_long_listening`, both panicking with `unexpected input SpottingLost` — the client found the command spotting service silent, pings included, for longer than its 5 s limit (`SILENCE_LIMIT` in `app/src/adapters/secondary/commandspotting/websocket/client.rs`). It is the first test binary cargo runs, and cargo stops at the first failing one, so the rest of `e2e-smoke` (`textrefinement_openai_it`, `transcription_openai_it`, `voice_forms_it`, `voice_samples_it`, `placed_commands_it`, the 137 `@smoke` scenarios) did not run in these runs.

**Verdict: sharing does not work today — the first criterion fails.**

- **No new failure, no restart in a concurrent pair — fails.** Every concurrent pair (5 of 5) had a failing run, 7 of 10 concurrent runs failed, always in `commandspotting_websocket_it`'s two "after long listening" tests, which passed in all 6 sequential runs. No supervisor restarted in any pair (0 lines, same PIDs). The bottleneck is the command spotting service under two runs' `commandspotting_websocket_it` at once (two times its 13 tests streaming at the same moment): the client loses the connection after 5 s without an answer. Not the speech recognition queue: the service's confirmations during pair 2 were answered in ≤ 370 ms (132 of them, mean 214 ms), so the silence lies in its streaming path (keyword spotting and VAD of every connection), not in whisper's queue.
- **Mean WER within 0.01 of the sequential median — holds.** 0.088 in all 16 runs, both at once (pairs 2 and 7) and beside the other run's `e2e-smoke` (pairs 4, 6, 8); "no command" passed in all 16.
- **Median concurrent pair ≤ 75 % of the median sequential pair — not judged.** Median sequential pair 626.4 s (625.9–631.9); median concurrent pair 314.3 s (50 %), but only because the failing runs skipped ≈ 245 s of `e2e-smoke`: no concurrent pair ran both runs in full. Where both runs' `e2e-quick` and `refine-eval` did overlap (pairs 2 and 7), they took 77–78 s for both runs against ≈ 94 s one after the other (≈ 82 %, above the 75 % bar): `refine-eval` nearly doubled (36.4–37.0 s against 18.6–19.7), as `llama-server`'s single slot (`-np 1`) predicts, and `e2e-quick` grew from ≈ 28 s to ≈ 40.5 s.

**No follow-up filed:** the criteria do not all hold. Before a later attempt, `commandspotting_websocket_it` would have to survive two runs at once (the service's streaming path under 26 connections, or the client's 5 s limit), and `llama-server` would need a second slot for the model stages to overlap usefully.

**What sharing cannot cover even when it works:** a branch that changes a service — `command-spotting/`, or a service's command line or model in the justfile — needs services of its own, built from that branch, on ports of their own; the `@local_services` and `@resilience` scenarios start their own services on free ports anyway.

**To repeat it:** with no `*.e2e` marker in `~/.voicepuncher/run/`, for each of two scratch worktrees `git worktree add --detach <dir> HEAD`, then in it `just _stages build build-e2e`; then for each pair, inside `~/.claude/bin/with-machine-lock bash -c '…'`, run in each worktree, one after the other or both in the background followed by `wait`:

```
VOICEPUNCHER_E2E_SERVICES_HELD=1 VOICEPUNCHER_BENCH_CORPUS=<main checkout>/e2e-test/corpus \
VOICEPUNCHER_VOICE_SAMPLES=<copy without session-2 27.23 s> \
  bash -c 'for s in e2e-smoke e2e-quick refine-eval; do just _$s; done'
```

timing each stage and counting the restart lines of the service logs before and after each pair (`just start-model-servers` first when the services are down, `just stop-model-servers` after the last pair).
