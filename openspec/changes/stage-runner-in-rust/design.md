## Context

See proposal.md § Why. The stage runner is spread over bash and Perl recipes that call each other through `just`; the self-tests are recipes too. Recipes outside this repository read parts of the `justfile`: the bundle scenario reads the stage lists of `check` and `full-check` through `_stages`, the app's catalog test reads the services' command lines, the docs check reads the recipe names.

## Decisions

- **One binary, `dev-tools/`** (workspace member `voicepuncher-dev-tools`, program `dev-tools`), called by the recipes through `cargo run --quiet -p voicepuncher-dev-tools --`. One subcommand per former helper recipe (`stages`, `model-stages`, `keys`, `stage-times`, `frozen`, `freeze-test`, `named-run`, `rerun-failed`, `accuracy-stage`, `gated-filter`, `engines`, `services`, `supervise`). It removes the variables `cargo run` sets (`CARGO_MANIFEST_DIR`, `CARGO_PKG_*`, …) from every child, so a build it starts sees the environment the recipe saw. Alternative: one shell script per helper — rejected, it keeps untested shell.
- **The stage lists stay in the `justfile`** (`check_model_stages`, `full_check_model_stages`, `e2e_smoke_stages`) and `_stages` stays a recipe that hands its stages to the binary: the bundle scenario and the documents keep reading them where they are.
- **The report between the runner and its locked phase keeps its text form** (`stage`, `reused`, `requested`, `locked`, `returned` lines), since the phase runs in another process under the lock wrapper.
- **Keys keep their algorithm**; the one change is that a directory outside the repository (a model folder, the corpus, the recordings) is walked in sorted order rather than in the order Perl's `File::Find` met it, so the key no longer depends on directory order. Golden tests take the former runner's keys for a fixture repository and environment without such directories. The test stage's part `test@justfile` goes: the self-tests are the binary's own tests, in its part `test@dev-tools`; a `justfile` edit still feeds every part as a file outside the members.
- **The services' start and stop move into the binary too**: with them in bash the `justfile` stays above 800 lines. The supervisor becomes `dev-tools supervise`, started detached in a process group of its own; its PID file, its log lines (`exited with code`, `stopped answering its health check`, `restarting in`), the ready lines and the copy lists stay as they were, which the local services scenarios read. The services' arguments are text constants in one source file that the catalog test reads instead of the `justfile`.
- **The machine lock is guarded, not vendored**: `_require-machine-lock` names the agent-kit install command and `VOICEPUNCHER_NO_MACHINE_LOCK=1`; the binary checks the same before a locked phase. The lock's own tests live in a `needs_machine_lock` module, left out and counted when the lock is absent (CI).
- **`|| true` is checked by a test** of the binary that reads the `justfile`: each one needs a comment on its line or the line above.
- **No `unsafe`**: file locks and signals come from `rustix` and `signal-hook`, already in `Cargo.lock`.

## Risks / Trade-offs

- [The first runs after the merge reuse no pass: the `justfile` feeds every key] → expected once, as after any `justfile` edit.
- [A behaviour of the bash runner lost in the port] → the former self-tests become Rust tests one by one, and `just check` runs the new runner on this branch before the landing queue's `full-check`.
- [`cargo run` adds a build step to every recipe] → a no-op build is ~0.2 s; the binary has few dependencies.

## Test levels

| Scenario | Level |
|---|---|
| The same inputs give the keys the former runner gave | unit (golden keys) |
| A run's journal line keeps its format | unit (golden line) |
| The summary reads the journal the former runner wrote | unit |
| The justfile is a thin layer | unit (reads the `justfile`) |
| A fresh clone without the lock tool gets the install command | unit |
| An uncommented tolerated failure fails the tests | unit |
| Without the machine lock the recipes run directly | unit (lock wrapper selection) |
| Model tests skip on a machine without models | existing tests, unchanged |
