## ADDED Requirements

### Requirement: A fast static check fails before any test runs

`just static` SHALL check formatting, compile the workspace with clippy's default lints, check the size and complexity budgets and check the complexity of the functions in the files changed against `main`, and SHALL stop at the first check that fails, naming it, without building or running a test. After a one-file edit it SHALL take about 10 s. `just test` SHALL run the same checks as its first stage and SHALL run no test when one of them fails.

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
