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

With `VOICEPUNCHER_NO_MACHINE_LOCK=1` the locked recipes SHALL run their bodies without the agent-kit lock, and `just test` SHALL leave out only the tests of the lock itself, counting them as skipped for the missing lock.

#### Scenario: Without the machine lock the recipes run directly

- **GIVEN** `VOICEPUNCHER_NO_MACHINE_LOCK=1` and no agent-kit lock on the machine
- **WHEN** `just fmt` runs
- **THEN** it runs `cargo fmt --check` without the lock wrapper
- **AND** `just test` reports the tests of the machine lock skipped and runs every other test

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

### Requirement: A test whose precondition is missing is skipped and counted, never green

A test that needs a service, a model, recordings or the desktop SHALL never pass by returning early when that need is missing. The recipe that runs it SHALL leave it out when the need is missing and SHALL print, before the tests run, how many tests it left out and which need was missing. A test the recipe runs SHALL fail, naming the missing need, when that need is missing after all.

#### Scenario: The unit run leaves out the tests that need the services and says so

- **GIVEN** no speech recognition or command spotting service is running for the run
- **WHEN** the developer runs the unit and integration tests
- **THEN** the tests that need those services do not run
- **AND** the run prints how many tests it left out for each missing need
- **AND** none of them is counted as passed

#### Scenario: A model stage runs the tests that need the services it started

- **GIVEN** a model stage that started the local services
- **WHEN** it runs its tests
- **THEN** the tests that need those services run and are judged on what they assert

#### Scenario: A test run without its need fails and names it

- **GIVEN** a test that needs the command spotting models
- **WHEN** it runs although the models are not configured
- **THEN** it fails with a message naming the missing models and the recipe that provides them

#### Scenario: Missing user recordings leave the recording tests out with a count

- **GIVEN** a machine without recordings of the user's voice
- **WHEN** the voice control stage runs
- **THEN** the tests on the user's recordings do not run
- **AND** the stage prints how many it left out because the recordings are missing

### Requirement: The desktop tests run in the full check when the permissions allow

The tests that use the real microphone, the Accessibility API and windows on the screen SHALL run as a stage of the full check, alone on the machine. When the process lacks a permission they need, the stage SHALL run none of them, name the missing permission, print how many tests it left out, show the stage as skipped rather than passed, and never record it as a pass; the user's manual run then runs them. Tests that press keys in the focused app SHALL never run in the full check.

#### Scenario: The full check runs the desktop tests with the permissions granted

- **GIVEN** the process running the full check has Microphone and Accessibility permission
- **WHEN** the full check runs
- **THEN** the desktop tests run in a stage of their own and its result counts like any other stage's

#### Scenario: A missing permission skips the desktop stage and names it

- **GIVEN** the process running the full check lacks Accessibility permission
- **WHEN** the full check reaches the desktop stage
- **THEN** the stage runs no desktop test and names Accessibility as missing
- **AND** the stage table shows the stage as skipped and the run prints the number of tests left out
- **AND** the run goes on and the stage is not recorded as a pass

#### Scenario: Key presses into the focused app stay out of the full check

- **GIVEN** the full check runs while the user works in another app
- **WHEN** the desktop stage runs
- **THEN** no test presses a key in the focused app

### Requirement: Every test asserts something that can fail

Every test function SHALL hold at least one check that can fail, and no test SHALL return before its first check. A test that only shows that code runs without a crash SHALL check an observable outcome instead or be removed. The architecture tests SHALL fail on a test without a check and on a test that returns before its first check.

#### Scenario: A test without a check fails the architecture tests

- **GIVEN** a test function whose body holds no check
- **WHEN** the architecture tests run
- **THEN** they fail and name that test

#### Scenario: An early return before the first check fails the architecture tests

- **GIVEN** a test that returns when an environment value is unset, before its first check
- **WHEN** the architecture tests run
- **THEN** they fail and name that test

### Requirement: Tests leave the user's clipboard alone

The tests of the paste SHALL write only a pasteboard of their own, never the general pasteboard the user copies to, so a test run while the user works never changes what the user copied.

#### Scenario: The paste tests run while the user has copied something

- **GIVEN** the user has copied a sentence
- **WHEN** the paste tests run
- **THEN** the user's clipboard still holds that sentence during and after the run

### Requirement: A step both harnesses use is written once

A Gherkin step text used by both the slice and component harness and the E2E harness SHALL be defined in one place shared by both, with one stated meaning; each harness SHALL only supply how its world performs or observes it. A step text defined separately in both harnesses SHALL fail the architecture tests.

#### Scenario: A step text defined in both harnesses fails the architecture tests

- **GIVEN** a step text that the slice and component steps and the E2E steps each define on their own
- **WHEN** the architecture tests run
- **THEN** they fail and name that step text

### Requirement: The stage runner is a tested program

The machinery behind the stage recipes (stage keys and reuse, the journal and its summary, lock phases, test ports, frozen tests, named runs, engines staging, the local services) SHALL live in one tested Rust program of the workspace; the `justfile` SHALL only name the stages and call it, hold no Perl and stay under 800 lines. The stage table, the journal line and the pass records SHALL keep their format.

#### Scenario: The same inputs give the keys the former runner gave

- **GIVEN** a scratch repository and environment whose stage keys the former runner computed
- **WHEN** the program computes the keys of the same stages
- **THEN** every key is the one the former runner computed

#### Scenario: A run's journal line keeps its format

- **GIVEN** the report of a run with a passed, a reused, a skipped and a failed stage, a lock wait and hold and a failed test in its output
- **WHEN** the program appends the run to the stage journal
- **THEN** the line holds the fields, their order and their number format the former runner wrote

#### Scenario: The summary reads the journal the former runner wrote

- **GIVEN** a stage journal with lines the former runner appended
- **WHEN** the developer runs `just stage-times`
- **THEN** the summary counts those runs, their stages, their lock holds and their failed tests

#### Scenario: The justfile is a thin layer

- **GIVEN** the `justfile` of the repository
- **WHEN** the tests run
- **THEN** it holds no Perl recipe and fewer than 800 lines

### Requirement: A missing machine lock names its install command

A recipe that runs under the machine-wide lock SHALL check that the lock tool is installed before it starts, and when it is missing SHALL fail naming the install command and the variable that runs the recipes without the lock, instead of failing on a missing file.

#### Scenario: A fresh clone without the lock tool gets the install command

- **GIVEN** a clone on a machine without the agent-kit lock tool and without `VOICEPUNCHER_NO_MACHINE_LOCK=1`
- **WHEN** the developer runs `just check`
- **THEN** it fails before its first stage
- **AND** the message names the command that installs the lock tool and `VOICEPUNCHER_NO_MACHINE_LOCK=1`

### Requirement: A helper's failure surfaces unless it is tolerated on purpose

A recipe of the `justfile` SHALL NOT discard a command's failure with `|| true` unless a comment beside it states which failure is tolerated and why; the tests SHALL fail on an uncommented one.

#### Scenario: An uncommented tolerated failure fails the tests

- **GIVEN** a recipe line that ends in `|| true` with no comment on it or on the line above
- **WHEN** the tests run
- **THEN** they fail and name that line

### Requirement: The accuracy baseline is recorded, accepted and shown as a trend
Every accuracy run SHALL record its sample size and mean word error rate in its stage journal line. `just accuracy-baseline` SHALL write the baseline file from this checkout's last journaled run of each sample size, with that run's commit, and keep the entry of a sample size this checkout never measured. The stage summary (`just stage-times`) SHALL show, per sample size, the mean word error rate of the last runs with their commits.

#### Scenario: An accuracy run is journaled with its rate
- **WHEN** an accuracy stage measures a mean word error rate of 0.047 over 100 recordings
- **THEN** the run's journal line records the sample size 100 and the rate 0.047

#### Scenario: The baseline is accepted from the last run
- **GIVEN** a stage journal whose last run of this checkout measured 0.04 over 10 recordings at commit `abc1234`
- **WHEN** the developer runs `just accuracy-baseline`
- **THEN** the baseline for 10 recordings is 0.04 from commit `abc1234`
- **AND** the baseline for 100 recordings is unchanged

#### Scenario: The summary shows the accuracy trend
- **GIVEN** a stage journal with three measured runs of 100 recordings
- **WHEN** the developer runs `just stage-times`
- **THEN** the summary lists the three rates of 100 recordings with their commits, oldest first

### Requirement: A fast static check fails before any test runs

`just static` SHALL check formatting, compile the workspace with clippy's default lints, check the size and complexity budgets and check the complexity of the functions in the files changed against `main`, and SHALL stop at the first check that fails, naming it, without building or running a test. In a local run (neither `CI` set nor the `full-check` recipe running it) it SHALL first apply clippy's machine-applicable fixes and format the workspace in place, SHALL print every file it changed, and SHALL then run the same checks; in a strict run (`CI` set, or `full-check`) it SHALL change no file and fail on what is unformatted or warned about. After a one-file edit it SHALL take about 10 s. `just test` SHALL run the same checks as its first stage and SHALL run no test when one of them fails.

#### Scenario: A function over budget fails the static check without a test

- **GIVEN** a function of 41 code lines that the size baseline does not list
- **WHEN** `just static` runs
- **THEN** it fails naming the function and its length
- **AND** no test was built or run

#### Scenario: A compile error fails the static check

- **GIVEN** a source file that does not compile
- **WHEN** `just static` runs
- **THEN** it fails naming the compile error
- **AND** the budget and complexity checks after it do not run

#### Scenario: A new complex function in a changed file fails the static check

- **GIVEN** a file changed against `main` holds a function with cyclomatic complexity 12 that the metrics baseline does not list
- **WHEN** `just static` runs
- **THEN** it fails naming the function and its complexity

#### Scenario: The test recipe stops at the static check

- **GIVEN** a function of 41 code lines that the size baseline does not list
- **WHEN** `just test` runs
- **THEN** it fails in its static stage
- **AND** its test stage does not run

#### Scenario: A clean tree passes the static check in seconds

- **GIVEN** one source file edited, formatted and within every budget
- **WHEN** `just static` runs
- **THEN** it passes within about 10 s

#### Scenario: A local static check fixes formatting and clippy findings and goes on

- **GIVEN** a source file that is not formatted and holds a clippy warning with a machine-applicable fix
- **WHEN** `just static` runs locally
- **THEN** the file is formatted and the warning fixed
- **AND** it names the file it changed
- **AND** it goes on to the budget and complexity checks

#### Scenario: A strict static check fixes nothing

- **GIVEN** a source file that is not formatted and holds a clippy warning with a machine-applicable fix
- **WHEN** `just static` runs with `CI` set
- **THEN** it fails on the formatting
- **AND** the file is unchanged

### Requirement: The full check runs only the stages an edit can affect

Each model stage of `just check` and `just full-check` SHALL be fed only by the files whose code it exercises, so that an edit outside them reuses the stage's earlier pass instead of running it. When no earlier pass with the same inputs exists, the stage SHALL run. A file no stage names SHALL feed every stage.

#### Scenario: An overlay change skips the seams, outages and start cues

- **GIVEN** `main` passed `just full-check`
- **AND** a branch that changes only the overlay's drawing code
- **WHEN** `just full-check` runs on the branch
- **THEN** the chunk seams, outages and start cue stages show as reused from `main`'s pass

#### Scenario: A change to the chunking code runs the seams stage

- **GIVEN** `main` passed `just full-check`
- **AND** a branch that changes how a dictation is cut into chunks
- **WHEN** `just full-check` runs on the branch
- **THEN** the chunk seams stage runs

#### Scenario: A file no stage names runs every stage

- **GIVEN** a branch that adds a source file no stage's inputs name
- **WHEN** `just full-check` runs on the branch
- **THEN** every model stage runs

### Requirement: The chunk seams stage runs a sample unless the chunking code changed

The chunk seams stage SHALL run a fixed sample of its seam cases when the branch leaves the chunking code unchanged, and the whole seam matrix when the branch changes it, on `main` itself and when the changed paths cannot be read. It SHALL print which of the two it runs before its first case.

#### Scenario: A branch outside the chunking code runs the seam sample

- **GIVEN** a branch that changes the voice control code but not the chunking code
- **WHEN** the chunk seams stage runs
- **THEN** it prints that it runs the sample and runs only the sample's cases

#### Scenario: A branch that changes the chunking code runs the whole matrix

- **GIVEN** a branch that changes the chunking code
- **WHEN** the chunk seams stage runs
- **THEN** it prints that it runs the whole matrix and runs every seam case

### Requirement: The app update survives a concurrent update of the main checkout

`just update-app` SHALL bring the main checkout to the newest remote `main` even while another program updates the same checkout: it SHALL wait, for about 30 seconds at most, while another git command holds the checkout, and SHALL count a checkout already at the newest `main` as updated. When the checkout cannot be updated, it SHALL stop before the rebuild with one line naming the blocker and what to do, never advising to discard work.

#### Scenario: Another program updates the checkout during the app update
- **GIVEN** the remote `main` is ahead of the main checkout
- **AND** another program holds the main checkout and then moves it to the newest `main` itself
- **WHEN** the user runs the app update
- **THEN** the update succeeds and the checkout is at the newest `main`

#### Scenario: A checkout behind main is moved forward
- **GIVEN** the remote `main` is ahead of the main checkout
- **WHEN** the user runs the app update
- **THEN** the update succeeds and the checkout is at the newest `main`

#### Scenario: A checkout already at the newest main is left as it is
- **GIVEN** the main checkout is already at the newest `main`
- **WHEN** the user runs the app update
- **THEN** the update succeeds and the checkout is unchanged

#### Scenario: Local changes stop the app update with one clear line
- **GIVEN** the remote `main` is ahead of the main checkout
- **AND** a tracked file of the main checkout has an uncommitted change
- **WHEN** the user runs the app update
- **THEN** the update fails with one line naming the local changes
- **AND** the output never suggests discarding work

#### Scenario: Another branch stops the app update with one clear line
- **GIVEN** the main checkout is on another branch
- **WHEN** the user runs the app update
- **THEN** the update fails with one line naming that branch

#### Scenario: Local commits not on the remote stop the app update with one clear line
- **GIVEN** the main checkout holds a commit the remote `main` does not have
- **WHEN** the user runs the app update
- **THEN** the update fails with one line naming the local commits

### Requirement: The architecture page shows every workspace crate

The project SHALL keep one offline architecture page, `docs/architecture/project-architecture.html`, whose crate diagram names exactly the members of the Cargo workspace; the project's checks SHALL fail on a member missing from the diagram or a diagram crate that is not a member, naming the crate.

#### Scenario: A workspace crate is missing from the page

- **GIVEN** the Cargo workspace has a member the page's crate diagram does not name
- **WHEN** the project's checks run
- **THEN** they fail, naming the missing crate

#### Scenario: The page names a crate that is not in the workspace

- **GIVEN** the page's crate diagram names a crate that is not a workspace member
- **WHEN** the project's checks run
- **THEN** they fail, naming the extra crate

#### Scenario: The page matches the workspace

- **GIVEN** the page's crate diagram names exactly the workspace members
- **WHEN** the project's checks run
- **THEN** the architecture page check passes
