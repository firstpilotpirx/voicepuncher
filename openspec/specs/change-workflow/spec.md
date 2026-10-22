# change-workflow Specification

## Purpose
How a change branch lands on main: through the landing queue, in groups checked once.

## Requirements

### Requirement: Branches land in groups through the landing queue

A change branch that changed a build input SHALL join the landing queue after its own `just check` passed, instead of being refused for being behind `main`. The queue leader SHALL merge up to four queued branches onto `main`, run one `just full-check` on the combined state and land every member of a passing group; a failing group SHALL be split until the failing branch is found, and the other members SHALL land.

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
