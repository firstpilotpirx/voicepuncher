## MODIFIED Requirements

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
