# Proposal

## Why

Several sessions run voicepuncher changes in parallel worktrees, and each `just check` (~6 min) and `just full-check` (~14 min) holds the machine-wide test lock (`with-machine-lock`) for its whole run, although only its E2E, accuracy and refinement stages use what makes the lock necessary: the local model services, the GPU and the shared ports. On 2026-10-15 until 15:54, 27 runs held the lock ≈ 145 min and waited for it ≈ 147 min; the longest wait was 27 min, and 11 waits took over 5 min (`research/README.md`).

Lock held per run, from the stage tables of every run on 2026-10-14/15 (`research/README.md`):

| Run | Lock held today | Lock held after this change | No longer under the lock |
|---|---|---|---|
| `check`, warm | ≈ 375 s | ≈ 300 s | fmt, lint, test, build ≈ 75 s |
| `check`, after an edit | ≈ 465 s | ≈ 300 s | the same stages ≈ 165 s |
| `full-check`, branch touches recognition | ≈ 920 s | ≈ 695 s | `cargo clean`, cold fmt, lint, test, build ≈ 225 s |
| `full-check`, branch does not touch recognition | ≈ 920 s | ≈ 595 s | the same ≈ 225 s, plus ≈ 100 s of WER: 10 recordings instead of 100 |

The last row is rare in this repository's history: applied to the 35 merges since 2026-09-15 that changed a build input, the recognition path list of `design.md` picks the 100 recordings for 34 — 28 through an app, service or test path or a dependency, 6 through the `justfile` alone; only UI and tooling branches such as the web-content focus fix (0.13.2) pick 10. The lock split carries most of the saving. The WER choice stays because the user asked for it; it saves ≈ 100 s per `full-check` where it applies and can be dropped later without touching the rest.

Two more costs come with today's checks. A run whose lint, tests or build fail learns it only after its wait. And `full-check` measures WER on 100 recordings (`e2e-bench`, 130 s median; 30 s for 10 recordings) on every branch, although on a branch that does not touch recognition it measures what the last recognition change already measured.

## What Changes

- **Lock only the model stages.** `check` and `full-check` are no longer locked whole. `cargo clean`, fmt, lint, `test` and the builds — `build`, and a new stage `build-e2e` that compiles the model stages' test binaries and the command spotting service's release build that `start-model-servers` runs — use none of the shared services, the GPU or shared ports, and run without the lock, alongside other runs. They keep a time limit: each runs through the lock wrapper with a lock directory of the run's own, so it starts at once and still ends with exit 124 when it hangs. A run waits for the lock when it reaches its first model stage — one that uses the services, the GPU or shared ports — and holds it once for all of them: E2E smoke (with the user's voice samples and the voice forms), the WER accuracy run, refinement evaluation and, in `full-check`, chunk seams and outages. It starts the services after taking the lock and stops the ones it started before giving it back, so no run stops a service another run uses. A run whose fmt, lint, tests or build fail ends without waiting for the lock.
- **WER on 100 recordings only when recognition changes.** `full-check` runs the 100-recording sample (`e2e-bench`) when the branch touches recognition, else the 10-recording one (`e2e-quick`, as `check` does). Recognition is: transcription and chunking; text refinement; the models and prompts; the service settings (the whisper-server, llama-server and HAProxy command lines in the `justfile`); the command spotting service and voice-command matching (the large run also checks 20 recordings of ordinary speech for commands, the small one 10); the WER and corpus test code; the dependencies (a version-only bump excluded). Design turns these areas into a path list; a path that can change what the large run measures belongs in it. When the branch's changes cannot be determined, or there are none (a check on `main` itself, as before a release), `full-check` runs the 100 recordings. A switch forces them when unsure, and works also when `kit` runs the check. The run prints which sample it chose and why.
- **Lock time in sight.** The stage table that ends `check` and `full-check` shows the lock wait and the lock hold as rows of their own, next to the stage times.
- **CPU sharing, an accepted risk.** One run's unlocked builds and tests now run while another run's model stages hold the lock, and compete with them for CPU and memory. The unlocked builds run under the utility QoS clamp (`taskpolicy -c utility`; `nice` does nothing on this macOS), so they yield the CPU to the model stages; the test binaries keep the default QoS, because clamped timer waits run 60–120 ms late under load, too late for their timing tests. The acceptance bar measures that the model stages stay green and nearly as fast under a parallel build.
- **Explore sharing the services between runs** — a measurement and a written finding, not a promise. Today an E2E stage reuses the services it finds on ports 8081, 8080 and 8082 and stops those it started by a marker per service, not per run, so two E2E runs cannot safely overlap. The measurement runs two E2E runs from two worktrees at once against one set of services — the three whisper-server copies behind HAProxy (`VOICEPUNCHER_STT_INSTANCES`), the text refinement service with its one slot, the command spotting service — several times, and the same two one after the other, under the machine lock so no other run is disturbed; it records failures, each run's time and the total. The finding goes into `research/README.md` and also says what sharing cannot cover (a branch that changes a service's build or command line needs services of its own). If sharing works without flakes and saves time, a follow-up change is filed with `create-change`; nothing of it is implemented here.
- **Docs.** `AGENTS.md` § Workflow and § How to verify a change, `README.md` § Testing and § Release checklist, `openspec/config.yaml` `context` (it still says `full-check` takes ~10 min) and the recipe comments say which stages take the lock, what `full-check` runs, how to force the 100 recordings, and the measured times.

No test is dropped or loosened: `check` runs the same tests (`build-e2e` only compiles what its model stages compiled themselves until now), and `full-check` runs every test it runs today on every branch — the accuracy run on 10 instead of 100 recordings when the branch does not touch recognition.

## Capabilities

### New Capabilities

None. This is check tooling, not app behavior: `skip_specs: true`.

### Modified Capabilities

None. The `file-transcription` requirement "Transcription meets the accuracy bar" keeps its 100 recordings and its bar; `full-check` runs its scenario on every branch that touches recognition, on `main` itself, and whenever forced or in doubt.

## Impact

- `justfile`: `check`, `full-check` and `_stages` (`_check` and `_full-check` fold into it; the lock around the model stages, the services started and stopped inside it, the time limit and QoS clamp of the unlocked stages, the lock rows of the stage table), new private recipes for the model stages, `build-e2e` and the choice of the WER sample with its switch, the build lines that the E2E recipes and `start-model-servers` share with `build-e2e`, the clamp of `_test`'s build phase, the comment on the lock at the top, recipe comments and times — among them the `e2e-quick` comment, which promises 20 recordings of ordinary speech where its limit gives 10. No Rust code changes: the app, the command spotting service and the tests stay as they are.
- `AGENTS.md`, `README.md`, `openspec/config.yaml`: as above. `.claude/kit.yaml` and the commands agents run (`just check`, `just full-check`) stay the same.
- `research/README.md` of this change: the measurement behind this proposal; the sharing finding is added to it.
- Overlap: `test-without-machine-lock`, filed separately, makes `just test` run without the machine lock. Both edit the lock wrapping in the `justfile` and the lock sentences of `AGENTS.md` and `README.md`, and both give unlocked work its time limit through the wrapper with a lock directory of its own; whichever merges second takes the other's edits along.

## Scope

- **Lifecycle stage:** PoC, stays PoC.
- **Acceptance bar:**
  - lock hold, measured as in `research/README.md` and read from the new rows of the stage table (median of three runs for `check`, one run per sample for `full-check`): `check` ≤ 345 s, warm and after an edit (expected ≈ 300 s); `full-check` ≤ 800 s with 100 recordings (expected ≈ 695 s) and ≤ 685 s with 10 (expected ≈ 595 s);
  - two `check` runs started together in two worktrees, each after an edit, run their fmt, lint, test and build stages at the same time, and each waits for the lock only after `build-e2e` (both logs show it);
  - while another worktree builds from scratch (its `cargo clean`, lint, `test` and build), the model stages of `check` pass in three runs out of three and take at most 15 % longer than the same stages alone on the same day;
  - `full-check` picks 100 recordings on a branch that touches a recognition path, 10 on a branch that does not, and 100 when forced, when the branch's changes cannot be determined or when there are none, each case shown once;
  - `just check` and `just full-check` pass on the final commit, bar failures that another filed change owns (named in the PR);
  - the sharing finding with its measurement is in `research/README.md`, and a follow-up change is filed when sharing works.
- **Out of scope:**
  - dropping `cargo clean` from `full-check`;
  - dropping or loosening a test or bar, moving tests between `check` and `full-check`, or changing which tests `check` runs;
  - implementing shared services between runs: only the exploration, and the follow-up change it may file;
  - `just test` run on its own without the lock: `test-without-machine-lock`; the lock of the other recipes run on their own (`fmt`, `lint`, `build`, the E2E recipes, `e2e`, `refine-eval`, `bench`, `bench-smoke`, `test-real-keyboard`, `release`) stays as it is;
  - kit changes (`kit` and its `with-machine-lock`);
  - a follow-up in agent-kit, not in this repo: `~/.claude/rules-custom/machine-lock.md` says to wrap every public build, test and check recipe whole in `with-machine-lock`; this change deliberately locks only the stages that use shared machine resources, so that rule needs an amendment there: lock the stages that use the shared services, the GPU or shared ports; builds and unit tests may run unlocked.

## Release

No release: no app behavior changes; `app/Cargo.toml` keeps its version (0.13.2 when filed).
