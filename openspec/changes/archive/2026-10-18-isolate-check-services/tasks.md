# Tasks

No spec scenario (`skip_specs: true`): each behavior is checked by running the recipes (design.md § Test levels); Red items show today's behavior first. In a worktree, set `VOICEPUNCHER_BENCH_CORPUS` to the main checkout's `e2e-test/corpus`. Never stop or restart a service this change's runs did not start.

## 1. Test ports, no reuse (D1, D3)

- [x] 1.1 Red: with the user's services on the default ports, run `just e2e-smoke` and record that it prints `reusing …` and tests the default ports; record `just check`'s lock time as the baseline.
- [x] 1.2 Add the test port set (and the commands port, copy ports) to the `justfile`, overridable by env vars; point every locked recipe's service URLs at it.
- [x] 1.3 Make `_start-model-servers-for-e2e` start every needed service on the test ports and mark each as its own; drop the reuse branch; `_stop-model-servers-for-e2e` stops exactly the marked ones.
- [x] 1.4 Print one line per started service: port, model file, PID.
- [x] 1.5 Green: rerun 1.1's setup; `just e2e-smoke` starts its own services on the test ports, passes, and the user's services keep their PIDs.
- [x] 1.6 `just test`; commit.

## 2. Held test ports (D2)

- [x] 2.1 Red: plant a listener on a test port (`nc -l 28081`); see today's recipe reuse or hang on it.
- [x] 2.2 Green: fail before the first stage with the message of D2, naming the holder's PID and command; the planted listener keeps running.
- [x] 2.3 Kill a locked run with KILL after its services start; the next run stops that leftover service and starts fresh.
- [x] 2.4 Update AGENTS.md § Local services, README and the justfile comments to say checks use ports of their own and never reuse.
- [x] 2.5 `just test`; `just check` with the user's services running on the default ports with another model; record its lock time against 1.1 for the PR; commit.

## 3. Version

- [x] 3.1 No version bump (no release). Manual checks: none.
