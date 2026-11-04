# Design

## Context

The runner has no desktop session, no pinned Homebrew and a Rust cache that prunes `target/`. The desktop tests are chosen by name: a Rust test under a `needs_<need>` module, which `just test` leaves out by a nextest name filter and `it-real` runs with the need's variable set. The slice/component harness shows nextest one test per `Rule:`, named by the feature file and the rule, so a Gherkin rule cannot carry a need yet.

## Goals / Non-Goals

**Goals:**
- One mechanism for every desktop test, Rust or Gherkin: the need in the test name.
- The workflow fails only on what the code does, never on what the runner installed or cached.

**Non-Goals:**
- Gating the scenario at run time with a `GATED:` line as `@real_keyboard` does: a desktop scenario left out by name must never pass green when run by name; its step fails naming the variable instead.
- Caching the sherpa-onnx download itself.

## Decisions

- **Analyzer binaries from their GitHub releases, not Homebrew.** The workflow reads each pin from the `justfile`, downloads that release's macOS arm64 archive and puts the binary into the tools folder the cache keeps, keyed by the pins. Homebrew installs whatever is current, and `_require-tools` rightly refuses it. `trufflehog_version` moves to 3.99.8, the release both machines get; the peer `speed-up-check-runtime` makes the same bump.
- **The sherpa-onnx build output is dropped when its library is gone.** The Rust cache keeps `target/*/build/` but prunes the folder the build script downloads into, so the cached output points at nothing and the link fails. One step after the cache restore removes the build script's output when the prebuilt folder is missing, so the script runs again (8.7 MB). Telling the cache to keep the folder was rejected: it prunes `target/` by its own list.
- **A `@needs_<need>` tag on a `Rule:` goes into its test name** as `<feature>/needs_<need>::<rule>`. The recipes' needs filter (`test(/needs_<need>::/)`) then leaves the rule out of `just test` and counts it; `it-real` runs it, with the harness among its targets and its filter without the leading `::`. The step of such a rule requires the desktop variable like a Rust desktop test. The real-keyboard recipe sets the desktop variable too, as it runs the whole harness. The peer moves the rule-naming helper to `test-steps`; the tag reading moves with it at the merge.
- **The watchdog tests push their audio in one call.** They paced the feed at real time; a loaded runner paces it slower than the silence limit. The watchdog's timers start from the last push, so the asserted bounds stay.
- **The fake hung program accepts every connection and keeps it open unanswered.** A listener that never accepts fills its backlog, after which macOS resets further connects and the probe reports the port closed, which is no hang. A real hung service has accepted the connection and says nothing.
- **The compile-error scenario runs the static check with colors off**: the step reads the compiler's message as text, and the workflow sets colors on for every command.

## Risks / Trade-offs

- [A tool release renames its archive] → the install step fails naming the URL; the pin bump that needs it fixes the name.
- [The runner's lint fails on an analyzer not yet seen running there] → the pull request's run shows it; fixed here (proposal, open point).
- [Kept connections of the fake hung program] → a few dozen per scenario, far below the descriptor limit.

## Test levels

| Spec scenario | Level |
|---|---|
| The full check runs the desktop tests with the permissions granted | component `.feature` (`change_workflow`), unchanged |
| A missing permission skips the desktop stage and names it | component `.feature`, unchanged |
| Key presses into the focused app stay out of the full check | component `.feature`, unchanged |
| A business rule marked for the desktop is a desktop test | component `.feature` (`change_workflow`) + unit test of the rule naming |
