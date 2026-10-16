# Design

## Context

See proposal.md § Why for the defect. What shapes the fix:

**cucumber-rs 0.23.0** (exact pin; gherkin 0.16.0, clap 4.6.1, regex 1.12.3 in `Cargo.lock`):

- `cli::Opts<Parser, Runner, Writer, Custom>` (`src/cli.rs`) has six public fields and is not `#[non_exhaustive]`:
  - `re_filter: Option<regex::Regex>`: `--name`, `-n`, alias `--scenario-name`;
  - `tags_filter: Option<gherkin::tagexpr::TagOperation>`: `--tags`, `-t`, also read from `CUCUMBER_FILTER_TAGS`, `conflicts_with = "name"`;
  - `parser`, `runner`, `writer`, `custom`.

  clap counts an environment value as explicit (`ValueSource::is_explicit` in clap_builder 4.6.0), so `--name` is refused beside `--tags` and beside `CUCUMBER_FILTER_TAGS`: at most one of the two fields is set.
- `Cucumber::with_cli(opts)` hands cucumber options parsed beforehand. `filter_run` (`src/cucumber.rs` 704–743) then applies `re_filter` if set, else `tags_filter` if set, else the harness's filter. With both `None`, the harness's filter is the only one.
- `--name` matches the scenario name (`Regex::is_match`, unanchored). `--tags` is evaluated over the feature's, the rule's and the scenario's tags; the parser has already merged an outline's Examples tags into each of its rows (`feature.rs`, `expand_examples`).
- The evaluator is public: `TagOperation::eval(tags)` through the sealed trait `cucumber::tag::Ext` (callable, not implementable). `TagOperation` implements `FromStr`, so a test can parse `"@smoke and not @wip"`. `cucumber::gherkin` re-exports gherkin.
- A step is skipped only when no step definition matches it (`runner/basic.rs`: `collection.find` returns `Ok(None)`); a defined step cannot skip itself. `fail_on_skipped()` turns a skipped step into a failure unless the scenario, its rule or its feature is tagged `@allow.skipped`.
- `--input <glob>` (`parser::basic::Cli`) replaces the features path the harness passes.
- `with_parser` drops options already given by `with_cli` (it sets `cli: None`).

**The harnesses today.** Both build their gates from the environment, print a `GATED:` line per closed gate and pass a filter to `filter_run_and_exit`; each keeps its own copy of `has_tag` (feature, rule and scenario tags). No `.feature` holds a `@wip` scenario. `@real_keyboard` tags `push_to_talk.feature` at feature level. `@voice_samples` tags five `@smoke` scenarios (16 runs) of the E2E `voice_control.feature`. A `harness = false` target runs no `#[test]`.

**The E2E harness library** (`voicepuncher-e2e-test`, `e2e-test/src/`) is a dependency of both harnesses: a dev-dependency of `app`, and the crate of `cucumber_e2e`. `cucumber` and `regex` are only its dev-dependencies. The workspace-wide `cargo build --release` of `_build` and `_release-package` builds its library.

**After `fail-hung-tests-fast`**, archived on `main` before this change starts. It is not implemented on 2026-10-16, so this design follows its design.md:

- a module of the E2E harness library reads each harness's command line: nextest's listing, or cucumber's command line from a suite's `run-extra-args` or from a direct run;
- the harness hands cucumber parsed options through `with_cli`;
- the suite selections, `and not @wip` included, live in `.config/nextest.toml`.

The selection it hands cucumber still replaces the gates.

## Goals / Non-Goals

**Goals:**

- The gates hold under every way cucumber selects: none, `--tags`, `--name`, `CUCUMBER_FILTER_TAGS`.
- A selection means what it means to cucumber today; only the gates are added to it.
- One unit-tested place decides for both harnesses, and the reproduction stays in `just test` as the regression test.

**Non-Goals:**

- Stopping `CUCUMBER_FILTER_TAGS` from narrowing a run. It is gated like `--tags`, nothing more.
- New gates, or a gate in a harness that does not have it today.
- Failing a run whose selection matches only gated scenarios.

## Decisions

### D1. The harness keeps the selection, cucumber gets options without it, and one filter requires both

Between parsing and `with_cli`, each harness passes `cli::Opts` and its gates to one function of the shared module (D2):

- It destructures the options naming all six fields, without `..`.
- It keeps `re_filter` and `tags_filter` for the filter.
- It returns the options with both fields `None`, plus the filter.

cucumber then applies only that filter, which runs a scenario when the selection matches it and every gate admits it. The exhaustive destructuring makes a cucumber release that adds a field to `Opts`, a new kind of filter for instance, fail to compile instead of slipping past the gates.

Rejected:

- **Runtime skip** (`fail_on_skipped`, `@allow.skipped`). cucumber-rs skips only undefined steps, so a gate step would pass, not skip. The scenario would count as passed while testing nothing, and the scenario count would not drop by the gated runs. `@allow.skipped` would also add tags to the scenarios and hide genuinely undefined steps in them.
- **Gates in the suites' `--tags`** (the `justfile` today, `run-extra-args` after fail-hung-tests-fast). `--name`, direct runs and `CUCUMBER_FILTER_TAGS` stay unguarded, short of the acceptance bar, and every selection repeats the gates.
- **Removing `--tags` and `--name` from the arguments before parsing.** It re-implements clap's spellings (`-t`, `--tags=…`, `-n`, `--scenario-name`) and misses `CUCUMBER_FILTER_TAGS`.
- **A gating parser**: `with_parser` around `parser::Basic`, dropping gated scenarios before cucumber's filter. It composes with any selection, but:
  - `with_parser` discards options already given by `with_cli`, an ordering trap beside fail-hung-tests-fast's `with_cli`;
  - its stream needs boxing;
  - with the gates out of the filter, the committed reproduction could only test a parser stub, not the composition the harnesses run today (D6).
- **Changing cucumber:** out of scope.

### D2. One module of the E2E harness library, used by both harnesses

`e2e-test/src/scenario_filter.rs`, exported from `lib.rs`, beside fail-hung-tests-fast's argument module.

| Part | Responsibility |
|---|---|
| `Gate` | a tag, open or closed, and its `GATED:` line when a variable closes it; built from the variable's value (D4) |
| `Selection` | everything, `Name(regex::Regex)` or `Tags(TagOperation)`, taken from the options |
| `ScenarioFilter` | `runs(feature, rule, scenario) -> bool`: the selection and every gate (D3); the `GATED:` lines of its closed gates |
| `split(opts, gates)` | returns the options without the selection, and the `ScenarioFilter`; the only way to get either |

- A harness keeps only its gates, the environment reads, printing the lines and the call. Both copies of `has_tag` go.
- `cucumber` (`=0.23.0`) and `regex` (`1`) move from `[dev-dependencies]` to `[dependencies]` of `voicepuncher-e2e-test`, unless fail-hung-tests-fast already moved them. `Cargo.lock` does not change.
- The module joins `recognition_paths` in the `justfile`, because it decides which `@accuracy` scenarios run (AGENTS.md: a branch that adds a file on the recognition path adds it there).
- Its tests live in its `#[cfg(test)] mod tests`, like those of the library's other modules.

Rejected:

- **Keeping the library free of cucumber**, with the selection passed in as closures. Each harness would again take the selection out of the options and build the matchers itself, which is the wiring that failed, twice. The tests could not use real tag expressions either.
- **The decision in each harness:** two copies that must not differ, and no unit test reaches a `harness = false` binary.

### D3. A selection means what it means to cucumber, and the gates read the same tags

- `Name(re)`: `re.is_match(&scenario.name)`.
- `Tags(op)`: `op.eval(…)` (`cucumber::tag::Ext`) over the feature's, the rule's and the scenario's tags, Examples tags included.
- Precedence is that of `filter_run`: the name, then the tags, then everything.
- `CUCUMBER_FILTER_TAGS` reaches `tags_filter` through clap and is gated like `--tags`.
- A gate looks for its tag among the same merged tags, as `has_tag` does today.
- A selection that matches only gated scenarios runs none of them and passes. The `GATED:` line says why, as it does today in `just test` for `@real_keyboard`.

### D4. The gates and their `GATED:` lines

| Harness | Tag | Runs when | Line when closed |
|---|---|---|---|
| both | `@wip` | never | none |
| `cucumber_e2e` | `@voice_samples` | `VOICEPUNCHER_VOICE_SAMPLES` is set and not empty | unset: `GATED: VOICEPUNCHER_VOICE_SAMPLES not set, skipping @voice_samples scenarios` (unchanged); empty: `GATED: VOICEPUNCHER_VOICE_SAMPLES empty, skipping @voice_samples scenarios` |
| `cucumber_integration` | `@real_keyboard` | `VOICEPUNCHER_REAL_KEYBOARD_IT` is set, to any value, as today | `GATED: VOICEPUNCHER_REAL_KEYBOARD_IT not set, skipping @real_keyboard scenarios` (unchanged) |

- Each harness keeps today's gates; neither learns the other's tag.
- Each line is plain text, like every `GATED:` line in the repo. It is printed once per closed variable gate, on stderr, before the scenarios run, whatever the selection, as today. It is not printed on nextest's listing, which prints only `scenarios: test`.
- The module takes the variable's value (the result of `std::env::var`) and holds the variable names; the harness reads the environment.

### D5. On top of fail-hung-tests-fast D4

After that change, each harness's `main` runs in four steps:

1. The argument module answers nextest's listing, or yields cucumber's command line.
2. The harness parses `cli::Opts` from it and builds its gates.
3. It calls `split(opts, gates)` and prints the lines.
4. It runs `…fail_on_skipped().with_cli(opts).filter_run_and_exit("tests/features", move |f, r, s| filter.runs(f, r, s))`.

The selections in `.config/nextest.toml` reach `split` as cucumber's command line and need no change.

If fail-hung-tests-fast landed differently, read both entry points on `main` at apply and keep one rule: every set of options handed to cucumber goes through `split`.

- Options parsed inside the argument module: `split` takes them as returned.
- No `with_cli`, so cucumber parses the process arguments itself: parse `cli::Opts` in the harness from the same arguments and add `with_cli`. Without it, cucumber cannot be handed options without the selection.
- `cucumber` already a dependency of the library: nothing to move.

### D6. Reproduction first, then the fix; the wiring proven on fixtures

**Red, one commit on its own.** The module, written with today's behavior:

- `split` returns the options untouched and a filter of the gates alone, today's closures copied;
- the line for an empty variable says "not set";
- the harness files stay untouched.

Its tests run cucumber in process:

- a test World with one step;
- a feature written to a temp dir, holding an ungated scenario, a `@wip`, a `@voice_samples` and a `@real_keyboard` scenario, with one gate tag on a `Rule` and one on an `Examples` block;
- options parsed from a command line, then `split` and `with_cli`;
- the names of the scenarios that ran, collected after each.

| Case | Expected | On the Red commit |
|---|---|---|
| gates closed, `--tags` selecting every fixture scenario | only the ungated scenario runs | fails: the gated scenarios run |
| the same with `--name` | the same | fails |
| `@wip` under `--tags @wip`, under `--name`, without a selection | no `@wip` scenario runs | fails under a selection |
| a gate open, a selection matching its scenario | that scenario runs | passes |
| no selection | what today's filter runs | passes |
| a selection that misses the ungated scenario | it stays out | passes |
| the voice-samples gate for unset, empty and a path | closed, closed, open, with the lines of D4 | fails for empty |

- The fixture text is test data in a plain Rust test; no `.feature` file is committed.
- The cases that pass on Red keep today's behavior by design. Each is seen failing once against a filter broken on purpose before it is trusted (AGENTS.md § TDD).
- Commit `test: reproduce scenario gates bypassed by --tags and --name`; its body says it is intentionally red.

**Green, one commit.**

- `split` takes the selection out, and the filter requires the selection and the gates.
- The empty-variable line.
- Both harnesses call the module.
- The module joins `recognition_paths`.

`just test` passes, and `cucumber_integration` without a selection runs as many scenarios as on the Red commit, whose harnesses are untouched. `just check` follows; the Red commit gets none.

**Fixture runs of the real harnesses**, on the Red commit (evidence that the unmodified binaries select gated scenarios) and on Green (the wiring). Each harness gets a temp feature, not committed:

- feature tag `@gates_fixture`;
- an ungated scenario tagged `@allow.skipped`;
- a `@wip` scenario and one with the harness's own gate tag, neither tagged `@allow.skipped`;
- step texts that match no step definition, so a selected scenario can only skip or fail on its step: no service, permission or key press, whatever the wiring.

They run through `kit run cargo test -p <package> --test <harness> -- --input '<dir>/*.feature' …`, without the machine lock (no service, port or GPU), with `VOICEPUNCHER_HANG_PROBE` unset.

| Run | On Green | On Red |
|---|---|---|
| gates closed (variables unset; `VOICEPUNCHER_VOICE_SAMPLES` empty), `--tags @gates_fixture`, then `--name fixture` | exit 0, `1 scenario (1 skipped)` | the `@wip` and gated fixture scenarios fail |
| gate open (`VOICEPUNCHER_REAL_KEYBOARD_IT=1`, `VOICEPUNCHER_VOICE_SAMPLES=<any path>`), `--tags` naming the gate | the gated fixture scenario is selected and fails on its undefined step | the same |

The real-feature runs of the Test levels table come only after the Green fixture runs pass. The agent never runs a `@real_keyboard` scenario: no `just test-real-keyboard`, and no real-feature run with `VOICEPUNCHER_REAL_KEYBOARD_IT` set.

Rejected:

- **A test that runs the harness binaries.** A test cannot find another test target's binary (cargo sets `CARGO_BIN_EXE_*` only for `[[bin]]` targets), and cargo nested in `just test` is slow and fragile.
- **Committed fixtures run by a recipe or a nextest profile in `just test`.** That is a second permanent run per harness to guard a two-line call, which the module's tests already cover in `just test`.
- **Routing the harnesses through the module in the Red commit.** It touches the code under test before the test (AGENTS.md § TDD).

### D7. What stays

- The suite selections keep `and not @wip`: the `justfile` today, `.config/nextest.toml` after fail-hung-tests-fast. The clause is redundant once the harnesses enforce `@wip`, not wrong, and leaving it keeps this change out of another change's fresh config. `_test-real-keyboard` gets `@wip` enforced without an edit.
- No `.feature`, step or tag changes; no recipe name or argument; no app code.
- README § Testing and AGENTS.md stay as they are: what they promise becomes true.

## Test levels

There is no spec scenario (`skip_specs: true`), so no `.feature` scenario is added, and there is no manual run by the user (proposal § Impact). Each acceptance item of proposal.md § Scope:

| Acceptance item | Level | How |
|---|---|---|
| The reproduction fails on today's code because a gated scenario is selected, and is committed alone as intentionally red | unit (E2E harness library, in-process cucumber run, D6) | Red commit: `kit run just test` fails only in the module's cases marked "fails" in D6, the selection cases naming the gated scenarios that ran. The fixture runs of the unmodified harnesses fail the same way; an excerpt goes in the PR. |
| Without recordings, `VOICEPUNCHER_VOICE_SAMPLES= just e2e-smoke` passes, runs no `@voice_samples` scenario, and says once why; its count drops by exactly the `@voice_samples` runs | recipe run by the agent (stage 6) | It passes with one `GATED: VOICEPUNCHER_VOICE_SAMPLES empty, …` line. Cucumber's `N scenarios` equals that of the run with recordings minus the `@smoke @voice_samples` runs counted in the features at apply time (16 today). |
| An unset variable counts as an empty one | unit | The voice-samples gate is closed for unset and for empty, open for a path; the two lines differ. |
| With recordings, `just e2e-smoke` runs the same scenarios and passes | recipe run (stage 6; this machine has the recordings) | `N scenarios` equals the same line in the last `e2e-smoke` or `check` log of `main` (kit run logs). |
| Without `VOICEPUNCHER_REAL_KEYBOARD_IT`, `cucumber_integration` under `--name paste` and under `--tags @component` runs no `@real_keyboard` scenario | unit, then harness runs (stage 6, after the fixture runs) | `kit run cargo test -p voicepuncher --test cucumber_integration -- --name paste`, then `-- --tags @component`: both pass, and "Hotkey press after a paste is registered" is not in their output. |
| Without recordings, `cucumber_e2e` under `--tags '@smoke and not @wip'`, or under a `--name` that matches a `@voice_samples` scenario, runs none of them | `--tags`: the recipe run above; `--name`: a harness run without services | `VOICEPUNCHER_VOICE_SAMPLES= kit run cargo test -p voicepuncher-e2e-test --test cucumber_e2e -- --name "user's recorded"`: `0 scenarios`, exit 0. Before the fix the same run fails at once in its first step, since no service URL is set. |
| No run of either harness runs a `@wip` scenario, whatever its selection | unit, fixture runs | The `@wip` cases of D6; each fixture holds a `@wip` scenario, which no `.feature` does today. |
| With the gate open, the same selection still runs the gated scenario (`just test-real-keyboard` selects its scenario) | unit, fixture run | The open-gate case. The fixture run with `VOICEPUNCHER_REAL_KEYBOARD_IT=1` and `--tags @real_keyboard` selects the fixture's scenario. `just test-real-keyboard` itself stays the user's release-checklist step. |
| Without a filter, both harnesses select what they select today; `just test` runs as many slice and component scenarios as before | unit, harness run, recipe run | The no-selection case. `kit run cargo test -p voicepuncher --test cucumber_integration`, without a selection, reports the same `N scenarios` on the Red commit (harnesses untouched) and on Green, where `just test` passes. nextest shows no output of a passing test, so the count is read from the direct run. |
| No bar is loosened | review | `git diff origin/main -- '*.feature'` is empty; no `@allow.skipped`, `@wip` or retry is added. |
| `just test` passes; `just check` after each task group; `just full-check` on the final commit | recipe runs | stages 5 and 8 |

## Risks / Trade-offs

- [fail-hung-tests-fast lands differently than its design] → D5 keeps one rule wherever the options are parsed; both entry points are read on `main` at apply.
- [A cucumber release changes `Opts` or `filter_run`] → the exact pin makes every bump deliberate; a new `Opts` field fails to compile (D1); the module's tests fail if cucumber filters on its own again.
- [cucumber in the library's `[dependencies]` joins the workspace release build] → no new crate and no change to `Cargo.lock`, and the app binary does not link it. Its tree builds beside the app's dependencies, then incrementally. The release-build stage of the first `just check` is compared with `main`'s, and the difference is reported in the PR.
- [A later edit bypasses the module in one harness] → `split` is the only way to get options without the selection; each harness's `main` says in its doc comment why cucumber never gets the selection; each fixture run is one command to repeat.
- [The Red module copies today's composition, so the committed reproduction runs a copy] → the fixture runs show the unmodified harnesses failing the same way on the Red commit; from Green on, both harnesses run exactly what the tests cover.
- [A fixture step matches a real step definition] → its text matches none. The closed-gate run comes first and shows the ungated fixture scenario skipped, which proves it.
- [A selection that matches only gated scenarios now passes with nothing run] → intended, and the `GATED:` line says why; a variable naming a directory without recordings still fails (out of scope).
- [`recognize-users-quiet-restart` lands first with more `@voice_samples` scenarios] → they are gated as written, and the expected drop is counted at apply time.

## Migration Plan

1. Once fail-hung-tests-fast is archived on `main`, branch from `origin/main` and read both harness entry points, the argument module and `e2e-test/Cargo.toml` there (D5).
2. Red commit (D6), then the fixture runs on it; their failure excerpt goes in the PR.
3. Green commit; `just test`; the fixture runs; `just check`.
4. Stage 6: the runs of the Test levels table, the real-feature runs last. Stage 8: `just full-check` with 100 recordings, since `cucumber_e2e.rs` and the `justfile` are on the recognition path.
5. Rollback: revert the squash commit. The harnesses return to their own filters, and the defect with them.
