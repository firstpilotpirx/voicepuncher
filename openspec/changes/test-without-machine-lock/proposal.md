# Proposal

## Why

Agents run `just test` after every edit (AGENTS.md § How to verify a change), and it waits for the machine-wide test lock (agent-kit `with-machine-lock`) like every check. With `/orchestrate`, parallel sessions are now the normal way to work through the backlog. On 2026-10-15 six voicepuncher changes ran in parallel sessions: their `just test` runs (~20 s each) queued behind other sessions' `check` and ~14-minute `full-check` runs, and the coding sessions sat idle. AGENTS.md promises that parallel changes run their checks one at a time "while coding goes on at once"; with `just test` locked, coding waits too. A waiter even gives up after an hour (exit 75), so a `just test` queued behind several `full-check` runs can fail without having run.

`just test` holds nothing the lock protects:

- **No shared service, no GPU.** It never gets the service URLs (`VOICEPUNCHER_TRANSCRIPTION_URL`, `VOICEPUNCHER_REFINEMENT_URL`, `VOICEPUNCHER_COMMAND_SPOTTING_URL`): the tests that need the shared speech recognition, text refinement or command spotting service skip, and nothing reaches ports 8080–8082 or runs a model on the GPU. Its stub services listen on ports the system picks (`127.0.0.1:0`).
- **No exclusive device.** Tests that need the real keyboard, pasteboard or microphone, or the Accessibility permission, run only with a `VOICEPUNCHER_REAL_*` variable, which `just test` never sets.
- **No shared build.** Each worktree builds in its own `target/` (cargo's default).

It does share the machine: CPU and memory, the cache of synthesized speech (`~/.cache/voicepuncher/speech/`; an entry joins it whole, so a parallel reader never sees half a file) and macOS `say` on a cache miss. The justfile exports the keyword spotting model paths to every recipe, so on a machine with the models `just test` also runs the command spotting service's `engine_it` and `server_it`: sherpa-onnx keyword spotting on the CPU, with stand-ins for the speech recognition service.

## What Changes

- **`just test` runs without the lock.** It starts at once, whatever else runs on the machine — a `check`, `full-check` or benchmark of this or another project, or another `just test` — and runs the same tests with the same bars as today.
- **Every other recipe keeps the lock:** `check`, `full-check` and `release` (their test stage runs under the lock they hold), `build`, `fmt`, `lint`, the E2E recipes (`e2e`, `e2e-smoke`, `e2e-quick`, `e2e-seams`, `e2e-resilience`, `e2e-bench`, `refine-eval`), `test-real-keyboard`, `bench` and `bench-smoke`.
- **`just test` keeps a time limit.** Today the lock wrapper kills a run after 3600 s with exit 124, which agents report as hung; an unlocked run that hangs is still killed at a time limit, with exit 124.
- **The docs say which recipes wait for the lock and why `just test` does not:** the justfile comment on the lock (today "every check, test and benchmark recipe"), the comment of `test`, and `AGENTS.md`.

The design settles what the lock no longer keeps away from `just test`:

- **CPU contention, both ways.** The lock's reason — two suites at once make both slow and flaky — still holds for the CPU. `just test` has real-time tests with tight bounds: the command spotting client's keep-alive, hang and reconnect tests (`commandspotting_client_it`, `commandspotting_reconnect_it`), `http_failure_it` (300 ms deadline), the `@serial` slice and component scenarios that watch the pipeline run (`text_insertion`, `voice_control`) and, with the models, `server_it` (pongs at most 1 s apart while a confirmation hangs) and `engine_it` (confirmation deadlines of 800 ms to 5 s). They now run beside a locked `check` or `full-check` and beside other `just test` runs. In turn, the build and the test binaries of `just test`, all started at once, take CPU from the locked run's real-time tests against the live services, such as `commandspotting_websocket_it` `should_serve_several_apps_listening_at_once`, which flaked once inside a `just check` on 2026-10-15.
- **`just test` beside its own worktree's `check` or `full-check`**, for example one started in the background: both use the same `target/`, and `full-check` begins with `cargo clean`. The lock kept them apart until now.

## Capabilities

### New Capabilities

None. This is test tooling, not app behavior: `skip_specs: true`.

### Modified Capabilities

None.

## Impact

- `justfile`: the `test` recipe, its comment and the comment on the lock.
- `AGENTS.md`: § Workflow ("Several changes at once") and § How to verify a change ("Local services") say that `just test` runs without the lock.
- `.claude/kit.yaml` needs no change: `kit verify after-edit` runs `just test` and takes no lock of its own.
- **Agent-kit follow-up, not part of this change:** the rule `~/.claude/rules-custom/machine-lock.md` says to wrap "every public recipe that builds, tests, checks or benchmarks" in `with-machine-lock`, and the agent-kit README (§ Machine-wide test lock) says the same. Both need a matching exception: a test recipe that uses no shared port, service, GPU or exclusive device may run without the lock. Until then, `test` departs from the rule on purpose.

## Scope

- **Lifecycle stage:** current PoC, target PoC — test tooling; the product stage does not move.
- **Scope:** the lock on `just test` and what running it unlocked brings; every other recipe, the bars of the tests and the lock itself stay as they are.
- **Acceptance bar:**
  - `just test`, started three times while another worktree's `just full-check` holds the lock — once in its `test` stage, once in its `build` stage, once in an E2E stage — starts at once each time (no `[machine-lock] held by …` wait) and passes; that `full-check` passes too.
  - Two `just test` runs started together from two worktrees both pass.
  - Every other recipe that waited for the lock still does: `check`, `full-check`, `release`, `build`, `fmt`, `lint`, the E2E recipes, `test-real-keyboard`, `bench`, `bench-smoke`.
  - A `just test` that hangs is killed at its time limit with exit 124.
  - `just check` and `just full-check` pass on the final commit.
- **Out of scope:**
  - unlocking `fmt`, `lint` or `build`: they touch no service either, but agents run them inside `check`, not after every edit, and the user named only `test`; they keep the lock;
  - loosening a bar (deadlines, `MAX_PONG_GAP`, tolerances) or dropping a test from `just test`;
  - the agent-kit rule and README exception, and any other kit change (`kit run`, `kit verify`, the lock);
  - sharing one `target/` or a build cache between worktrees.

## Release

No release: no app behavior changes; `version` in `app/Cargo.toml` stays as it is (0.13.2 today).
