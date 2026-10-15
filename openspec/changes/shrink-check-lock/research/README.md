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

The E2E recipes also compile their test binaries (`cucumber_e2e`, the `*_it` binaries of `e2e-smoke`, `placed_commands_it`) inside the locked stage: ≈ 5 s warm, more after an edit of `e2e-test/` or a test.

## How long runs waited

The wait is a run's wall time minus the sum of its stage times (for `full-check` this includes `cargo clean`, ≈ 20–60 s). On 2026-10-15 until 15:54:

| Recipe | Runs | Waited in total | Median wait | Longest wait | Waits over 5 min |
|---|---|---|---|---|---|
| `check` | 19 | 94 min | 75 s | 27 min | 7 |
| `full-check` | 8 | 52 min | 6.8 min | 21 min | 4 |

Together the runs held the lock for ≈ 145 min and waited for it ≈ 147 min. Every second a run holds the lock without needing it is a second every queued run waits.

## Sharing the speech recognition service between two runs (not measured yet)

Today every E2E stage reuses services it finds on ports 8081/8080/8082 and stops only those it started itself (marker files per service, not per run): a run that started the services stops them at its end even while another run still uses them. `llama-server` runs one slot (`-np 1`); the speech recognition service is three `whisper-server` copies behind HAProxy (`leastconn`, one request per copy, queue). Whether two E2E runs can share them without flakes is the open question of task group "explore sharing".
