## ADDED Requirements

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
