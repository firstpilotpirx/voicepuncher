# change-workflow Specification

## Purpose
How a change branch lands on main: through the landing queue, in groups checked once.

## Requirements

### Requirement: Branches land in groups through the landing queue

A change branch that changed a build input SHALL join the landing queue after its own `just check` passed, instead of being refused for being behind `main`. The queue leader SHALL merge up to seven queued branches onto `main`, run one `just full-check` on the combined state and land every member of a passing group; a failing group SHALL be split until the failing branch is found, and the other members SHALL land.

#### Scenario: A checked branch behind main joins the queue

- **GIVEN** a branch whose own `just check` passed
- **AND** `main` moved after that check
- **WHEN** the session lands the branch
- **THEN** the branch joins the landing queue instead of being sent to check again

#### Scenario: A group lands after one full check

- **GIVEN** three queued branches, each with a passing `just check`
- **WHEN** the queue leader lands the group
- **THEN** one `just full-check` runs on the three branches merged onto `main`
- **AND** all three land on `main` once it passes

#### Scenario: Seven queued branches land as one group

- **GIVEN** seven queued branches, each with a passing `just check`
- **WHEN** the queue leader forms the next group
- **THEN** the group holds all seven branches
- **AND** an eighth queued branch waits for the next group

#### Scenario: A red group lands its green members

- **GIVEN** a queued group in which one branch breaks the full check
- **WHEN** the group's full check fails
- **THEN** the group is split until the breaking branch is found
- **AND** the other branches land
- **AND** the breaking branch is sent back to its session with the failure

#### Scenario: A branch without build inputs needs no full check

- **GIVEN** a branch that changed only docs or OpenSpec documents
- **WHEN** the session lands it
- **THEN** it lands after `just test` without waiting for a group's full check

### Requirement: Code stays within size and complexity budgets

The project's checks SHALL fail on a unit of Rust code over a size or complexity budget (function 40 code lines, 5 parameters, nesting 4, cognitive complexity 15, file 300 code lines, struct 12 fields, type 35 public members, module directory 2 000 code lines) unless the size baseline lists it. A listed unit SHALL fail them when it grows past its listed measure or falls within its budget while still listed.

#### Scenario: A new function over budget fails the checks

- **GIVEN** a function of 41 code lines that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the function and its length

#### Scenario: A listed unit that grew fails the checks

- **GIVEN** a function the size baseline lists with 52 code lines
- **AND** the function now has 53 code lines
- **WHEN** the checks run
- **THEN** they fail and name the function, its listed and its current length

#### Scenario: A listed unit that shrank keeps passing

- **GIVEN** a function the size baseline lists with 52 code lines
- **AND** the function now has 45 code lines
- **WHEN** the checks run
- **THEN** they pass

#### Scenario: A listed unit back within budget must leave the baseline

- **GIVEN** a file the size baseline lists with 420 code lines
- **AND** the file now has 280 code lines
- **WHEN** the checks run
- **THEN** they fail and ask for the file's entry to be removed from the baseline

#### Scenario: A struct with too many fields fails the checks

- **GIVEN** a struct with 13 fields that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the struct and its field count

#### Scenario: A type with too large a public surface fails the checks

- **GIVEN** a type with 36 public methods that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the type and its public member count

#### Scenario: A module directory over budget fails the checks

- **GIVEN** a module directory of 2 001 code lines that the size baseline does not list
- **WHEN** the checks run
- **THEN** they fail and name the directory and its size

#### Scenario: Comments and blank lines do not count

- **GIVEN** a function of 30 code lines with 20 comment and blank lines between them
- **WHEN** the checks run
- **THEN** the function counts as 30 code lines and passes

### Requirement: Clippy findings are ratcheted in the lint stage

`just lint` SHALL run clippy's `pedantic` and `nursery` groups and the restriction lints `unwrap_used`, `expect_used`, `panic` (outside test code) and `undocumented_unsafe_blocks`. A finding the baseline does not list, or a listed count that grew, SHALL fail the stage and name it; a listed count that shrank SHALL pass; a listed finding that is gone SHALL fail the stage until its entry leaves the baseline.

#### Scenario: A new clippy finding fails the lint stage

- **GIVEN** a production function that calls `unwrap()` in a file the baseline lists with no `unwrap_used` finding
- **WHEN** the lint stage runs
- **THEN** it fails and names the lint, the file and the count

#### Scenario: A listed finding that shrank keeps passing

- **GIVEN** a file the baseline lists with 12 `cast_precision_loss` findings
- **AND** the file now has 10
- **WHEN** the lint stage runs
- **THEN** it passes

#### Scenario: A finding that is gone must leave the baseline

- **GIVEN** a file the baseline lists with 3 `float_cmp` findings
- **AND** the file now has none
- **WHEN** the lint stage runs
- **THEN** it fails and asks for the entry to be removed from the baseline

### Requirement: Dependencies, spelling and recipes are analyzed in the lint stage

`just lint` SHALL check for unused dependencies, the dependency policy (advisories, licenses, duplicate versions, sources), the semver of the `copy-pool` library and the spelling of the repository, and SHALL run shellcheck and perlcritic over the `justfile` recipe bodies against a baseline. A missing or wrong-version analyzer SHALL fail the stage with the command that installs it; the stage SHALL install nothing.

#### Scenario: A shell mistake in a recipe fails the lint stage

- **GIVEN** a recipe body with a shellcheck finding the baseline does not list
- **WHEN** the lint stage runs
- **THEN** it fails and names the recipe and the shellcheck code

#### Scenario: A missing analyzer names its install command

- **GIVEN** a machine without the pinned spelling checker
- **WHEN** the lint stage runs
- **THEN** it fails before any analyzer runs and prints the command that installs the pinned version

### Requirement: Code metrics are reported and ratcheted

`just metrics`, which `just test` runs, SHALL print the 25 functions of highest cyclomatic complexity and a table per module (function count, function lengths, highest complexity, `unsafe`/`unwrap`/`clone` counts, fan-in and fan-out). It SHALL fail on a function over cyclomatic 10 the baseline does not list or that grew, on an unlisted import cycle, on a crate whose `unsafe` count grew, and on more duplicated lines than listed.

#### Scenario: A new complex function fails the metrics

- **GIVEN** a function with cyclomatic complexity 12 that the metrics baseline does not list
- **WHEN** `just metrics` runs
- **THEN** it fails and names the function and its complexity

#### Scenario: Copy-pasted code fails the metrics

- **GIVEN** the baseline lists 300 duplicated lines
- **AND** a new copy of a 40-line function
- **WHEN** `just metrics` runs
- **THEN** it fails and names the duplicated lines found and the baseline's

#### Scenario: The report lists the worst functions and the modules

- **WHEN** `just metrics` runs
- **THEN** it prints the 25 most complex functions and one row per module

### Requirement: Every pull request has a hosted check

A GitHub Actions workflow SHALL run `just fmt`, `just lint` and `just test` on a macOS Apple Silicon runner for every pull request and every push to `main`, with no model, service or machine lock, leaving out only the tests that need the owner's desktop session or voices; any failing step SHALL fail the check. The landing queue's `full-check` SHALL stay the merge gate for build inputs.

#### Scenario: A formatting error turns the check red

- **GIVEN** a pull request with a file `cargo fmt` would change
- **WHEN** the hosted check runs
- **THEN** the check fails at the formatting step

#### Scenario: A failing unit test turns the check red

- **GIVEN** a pull request with a unit test that fails
- **WHEN** the hosted check runs
- **THEN** the check fails at the test step and names the test

#### Scenario: A clean pull request turns the check green

- **GIVEN** a pull request that formats, lints and tests clean
- **WHEN** the hosted check runs with its caches warm
- **THEN** the check passes within 10 minutes

### Requirement: Recipes run without the machine lock when told so

With `VOICEPUNCHER_NO_MACHINE_LOCK=1` the locked recipes SHALL run their bodies without the agent-kit lock, and `just test` SHALL skip only the lock's own self-test, saying so.

#### Scenario: Without the machine lock the recipes run directly

- **GIVEN** `VOICEPUNCHER_NO_MACHINE_LOCK=1` and no agent-kit lock on the machine
- **WHEN** `just fmt` runs
- **THEN** it runs `cargo fmt --check` without the lock wrapper
- **AND** `just test` reports the machine lock self-test skipped and runs every other test

#### Scenario: Model tests skip on a machine without models

- **GIVEN** a machine without the command spotting models, where the `justfile` exports their paths empty
- **WHEN** `just test` runs
- **THEN** the command spotting model tests skip and pass

### Requirement: Every unsafe site states why it is sound

The project's checks SHALL fail on an `unsafe` block or `unsafe impl` without a `// SAFETY:` comment directly above it, and on an `unsafe` block holding more than one unsafe operation, a single `msg_send!` counting as one. They SHALL fail on an `unsafe impl Send` or `unsafe impl Sync` that the allowlist of thread-crossing types does not list with its file and its reason, and on an allowlist entry that no longer matches an `unsafe impl` in the code.

#### Scenario: An unsafe block without a SAFETY comment fails the checks

- **GIVEN** an `unsafe` block with no `// SAFETY:` comment above it
- **WHEN** the checks run
- **THEN** they fail and name the block's file and line

#### Scenario: An unsafe block with two unsafe operations fails the checks

- **GIVEN** an `unsafe` block that calls two unsafe functions
- **WHEN** the checks run
- **THEN** they fail and name the block's file and line

#### Scenario: A new unsafe impl Send outside the allowlist fails the checks

- **GIVEN** an `unsafe impl Send` for a type the allowlist does not list
- **WHEN** the checks run
- **THEN** they fail and name the type and its file

#### Scenario: A listed unsafe impl Send keeps passing

- **GIVEN** an `unsafe impl Send` for a type the allowlist lists with its file and reason
- **AND** a `// SAFETY:` comment above it
- **WHEN** the checks run
- **THEN** they pass

#### Scenario: A stale allowlist entry fails the checks

- **GIVEN** an allowlist entry whose type has no `unsafe impl Send` or `Sync` left in its file
- **WHEN** the checks run
- **THEN** they fail and ask for the entry to be removed

### Requirement: Documents name only things that exist

The project's checks SHALL fail when a document written for people or agents (`AGENTS.md`, `README.md`, `docs/**/*.md`) names a `just` recipe, a repository path or a loopback port that does not exist in the repository, naming the document, the line and the missing thing; a document whose every named recipe, path and port exists SHALL pass.

#### Scenario: A document names a recipe that does not exist

- **GIVEN** a document names a task-runner recipe the task runner does not list
- **WHEN** the project's checks run
- **THEN** they fail, naming the document, the line and the recipe

#### Scenario: A document names a path that does not exist

- **GIVEN** a document names a repository file or folder that is not in the repository
- **WHEN** the project's checks run
- **THEN** they fail, naming the document, the line and the path

#### Scenario: A document names a port the code does not use

- **GIVEN** a document names a loopback port that no service default in the code uses
- **WHEN** the project's checks run
- **THEN** they fail, naming the document, the line and the port

#### Scenario: Every named thing exists

- **GIVEN** every recipe, path and port the documents name exists
- **WHEN** the project's checks run
- **THEN** the documents check passes

### Requirement: Integration tests build into one binary per member

The integration tests of a workspace member SHALL build into one test binary, besides the Gherkin harnesses and the tests that must run on the main thread, so that an edit relinks one binary per member instead of one per test file. The checks SHALL fail on a test file under that binary's directory that the binary does not include, naming the file. Every test SHALL keep its own process, its time limit, its test group and its place in the run order.

#### Scenario: A one-file edit relinks one binary per member

- **GIVEN** a workspace whose test targets are built
- **WHEN** one source file of the application is edited and the test targets are built again
- **THEN** the application's integration tests are linked once, as one binary
- **AND** the Gherkin harnesses and the main-thread tests are linked as their own binaries

#### Scenario: A test file the binary leaves out fails the checks

- **GIVEN** a new test file under a member's integration test directory
- **AND** the member's integration test binary does not declare it
- **WHEN** the checks run
- **THEN** they fail and name the file

#### Scenario: A moved test keeps its isolation and limits

- **GIVEN** a test that drove a shared service one at a time before the move
- **WHEN** the test suite runs after the move
- **THEN** that test still runs one at a time, in a process of its own, within its time limit
- **AND** the suite runs the same tests as before the move

### Requirement: One named test runs alone

The project SHALL offer one recipe that runs only the tests and slice or component scenarios it is given by name, with the same test runner, configuration and time limits as `just test`, unlocked; it SHALL fail when a given name matched no test or scenario that ran, and it SHALL be recorded in the stage journal under its own recipe name, never as a pass of a stage.

#### Scenario: A named unit test runs alone

- **GIVEN** a workspace with many tests and one new test named by its name
- **WHEN** the developer runs the one-test recipe with that name
- **THEN** only the tests whose name matches it run
- **AND** the run reports that test's result within seconds of the build

#### Scenario: A named scenario runs alone

- **GIVEN** a slice or component feature file holding a scenario named by its name
- **WHEN** the developer runs the one-test recipe with that scenario's name
- **THEN** only that scenario runs, and no other scenario of the harness

#### Scenario: A name that matches nothing fails the run

- **GIVEN** a name that matches no test and no scenario
- **WHEN** the developer runs the one-test recipe with it
- **THEN** the run fails, naming the name that never ran

#### Scenario: A one-test run is journaled but never a pass

- **GIVEN** a one-test run that passed
- **WHEN** the next `just test` runs on the same inputs
- **THEN** the stage journal holds the one-test run under its own recipe name
- **AND** `just test` reuses nothing from it and runs its stages

### Requirement: The Red step of TDD runs only its new test

A TDD Red step SHALL run only the new test that is meant to fail, through the one-test recipe; the Green and Refactor steps SHALL run that same test until it passes; `just test`, which runs only the packages whose inputs changed, SHALL run once per task group, before its commit, not after every edit.

#### Scenario: Red runs one test, not the suite

- **GIVEN** a task whose Red step adds one failing test
- **WHEN** the agent runs the Red step
- **THEN** it runs that one test alone and sees it fail for the expected reason
- **AND** the stage journal shows no `just test` run for that step

#### Scenario: The full suite runs once per task group

- **GIVEN** a task group whose tasks are all green
- **WHEN** the agent finishes the group
- **THEN** it runs `just test` once before committing the group

### Requirement: E2E stages run alone on the machine

Every model stage of a stage run (`check`, `full-check`, the E2E recipes) SHALL hold the machine-wide test lock exclusively, so no two runs run model stages at once, and the build and test stages of every other run, standalone `just test` included, SHALL not run while it holds it. The stage table and the stage journal SHALL show the wait for that lock and its hold, and the journal SHALL record when each hold began. The stage summary (`just stage-times`) SHALL report whether the model-stage holds of two runs overlapped: that none did, or each overlapping pair of runs.

#### Scenario: Two runs' model stages take turns

- **GIVEN** a run whose model stages hold the machine lock
- **WHEN** another run reaches its model stages
- **THEN** it waits until the first run returns the lock, and only then starts its model stages

#### Scenario: A build waits while model stages run

- **GIVEN** a run whose model stages hold the machine lock
- **WHEN** another session starts `just test` or a check reaches its build stages
- **THEN** that stage starts only after the model stages returned the lock

#### Scenario: Model stages wait for running builds and keep new ones out

- **GIVEN** a build stage of another run is running
- **WHEN** a run asks the lock for its model stages, and another build stage starts after that
- **THEN** the model stages start only after the running build ended
- **AND** the later build starts only after the model stages returned the lock

#### Scenario: Builds still run beside each other

- **GIVEN** no run holds the lock for its model stages
- **WHEN** two sessions run build or test stages at once
- **THEN** both run at the same time

#### Scenario: The journal shows the lock wait and hold

- **WHEN** a run with model stages ends
- **THEN** its stage table and its journal line show how long it waited for the lock and how long it held it
- **AND** its journal line shows when the hold began

#### Scenario: The stage summary finds no overlap between runs that took turns

- **GIVEN** journal lines of runs whose model-stage holds followed one another
- **WHEN** the user summarizes the stage journal
- **THEN** the summary says that no two runs' model-stage holds overlapped

#### Scenario: The stage summary names overlapping runs

- **GIVEN** journal lines of two runs whose model-stage holds overlapped
- **WHEN** the user summarizes the stage journal
- **THEN** the summary names both runs, by branch and start time, and how long they overlapped

### Requirement: Up to seven changes run at once

The orchestrator SHALL run up to seven changes of this project at once, each in its own session and worktree; a change past that limit SHALL wait until a running change ends.

#### Scenario: Seven ready changes start together

- **GIVEN** seven ready changes and none running
- **WHEN** the orchestrator asks which changes it may launch
- **THEN** all seven are launchable

#### Scenario: An eighth change waits for a free slot

- **GIVEN** seven running changes and one more ready change
- **WHEN** the orchestrator asks which changes it may launch
- **THEN** the eighth change is not launchable until a running change ends

### Requirement: A test that flakes outside the change is frozen, not filed

When a check fails only in a test the branch did not touch, the session SHALL freeze that test with one command and land the change, filing no change, declaring no dependency and never rerunning the test unchanged. The freeze SHALL record where and when the test failed and SHALL reach `main` at once. A failure in a test the branch touched SHALL stay the change's to fix.

#### Scenario: An out-of-scope failure is frozen and the change lands

- **GIVEN** a change whose check failed only in a scenario the branch did not touch
- **WHEN** the session freezes that scenario
- **THEN** the scenario is listed as frozen on the branch and on `main`, with the day, recipe, stage and what ran beside it
- **AND** no new change is filed and the change declares no dependency
- **AND** the check runs again without the frozen scenario, reusing the stages that already passed

#### Scenario: A failure in a test the change touched is not frozen away

- **GIVEN** a change whose check failed in a test whose file the branch changed
- **WHEN** the session handles the failure
- **THEN** it fixes the failure in the change and freezes nothing

### Requirement: A frozen test runs nowhere until it is unfrozen

A frozen test SHALL run in no recipe — `just test`, `just check`, `just full-check`, the E2E recipes — in any worktree whose list of frozen tests, merged with the list on `main`, names it; the harness SHALL say which scenarios it skipped as frozen. A branch that removes a test from the list SHALL run that test again, even while `main` still lists it.

#### Scenario: A frozen scenario is skipped and named

- **GIVEN** a scenario on the frozen list
- **WHEN** a recipe runs the feature that holds it
- **THEN** that scenario does not run
- **AND** the run's output names it as skipped because it is frozen
- **AND** the feature's other scenarios run

#### Scenario: A frozen plain test does not run

- **GIVEN** an integration test on the frozen list
- **WHEN** `just test` runs
- **THEN** that test does not run and every other test runs

#### Scenario: A freeze on main reaches another worktree

- **GIVEN** a worktree whose own list does not name a test
- **AND** `main` lists that test as frozen
- **WHEN** a recipe runs in that worktree
- **THEN** the test does not run there

#### Scenario: The fixing branch unfreezes the test

- **GIVEN** a branch that removed a test from the frozen list
- **AND** `main` still lists it
- **WHEN** a recipe runs on that branch
- **THEN** the test runs

### Requirement: The frozen tests are listed and run on demand

`just frozen-tests` SHALL list every frozen test with the day, recipe, stage and what ran beside it, and `just frozen-tests run` SHALL run only the frozen tests, each in the stage it failed in. Every check run SHALL print how many tests are frozen and name those frozen for more than 7 days.

#### Scenario: The user lists the frozen tests

- **GIVEN** two frozen tests
- **WHEN** the user lists the frozen tests
- **THEN** both are shown, each with its day, recipe, stage and what ran beside it

#### Scenario: The user runs only the frozen tests

- **GIVEN** a frozen scenario of the voice control smoke stage
- **WHEN** the user runs the frozen tests
- **THEN** only that scenario runs, in its stage, with that stage's services and lock

#### Scenario: A check names tests frozen too long

- **GIVEN** one test frozen 3 days ago and one frozen 9 days ago
- **WHEN** a check runs
- **THEN** it prints that 2 tests are frozen
- **AND** it names the one frozen 9 days ago as frozen for more than 7 days
