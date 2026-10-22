# Spec Delta

## ADDED Requirements

### Requirement: Test coverage is reported and ratcheted in the full check

The full check SHALL measure the line and region coverage (a region is a stretch of code with one control-flow path) the unit, integration and architecture tests reach in each workspace crate, without models or services, print one row per crate and record it in the stage journal. A baseline SHALL hold a floor per crate for lines and regions with the commit it came from; a crate under its floor SHALL fail the stage and name the crate, the floor and the value.

#### Scenario: A crate that lost coverage fails the full check

- **GIVEN** the coverage baseline lists the core crate with 78 % of lines covered
- **AND** the branch's tests cover 76 % of its lines
- **WHEN** the full check runs
- **THEN** the coverage stage fails and names the crate, the floor and the value

#### Scenario: A crate that gained coverage keeps passing

- **GIVEN** the coverage baseline lists the core crate with 78 % of lines covered
- **AND** the branch's tests cover 81 % of its lines
- **WHEN** the full check runs
- **THEN** the coverage stage passes and prints the new value

#### Scenario: The coverage report reaches the journal

- **WHEN** the full check runs
- **THEN** the stage journal's entry carries the line and region coverage of every crate
- **AND** the report file is kept in the machine's cache beside the stage journal

### Requirement: The mutation score is reported and ratcheted in the full check

The full check SHALL run mutation testing (small changes to production code that a test must catch) without models or services: the whole copy-pool library, and the core layers only in the lines the branch changed. It SHALL print per crate the mutants tried, caught and survived and every survivor. A baseline SHALL hold a floor per crate of the share caught, with its commit; a crate under its floor SHALL fail the stage naming the crate, the floor and the value.

#### Scenario: A crate whose tests let more mutants through fails the full check

- **GIVEN** the mutation baseline lists the copy-pool library with 90 % of mutants caught
- **AND** the branch's tests catch 85 % of the mutants run for it
- **WHEN** the full check runs
- **THEN** the mutation stage fails and names the crate, the floor and the value

#### Scenario: Core mutants of unchanged lines do not run in the full check

- **GIVEN** a branch that changed one function of the core
- **WHEN** the full check runs
- **THEN** the mutation stage tries only the mutants of that function's changed lines against the core's floor
- **AND** the copy-pool library's mutants all run

#### Scenario: Survivors are listed for the next tests

- **GIVEN** a production function in the branch's changed lines whose return value no test checks
- **WHEN** the mutation stage runs
- **THEN** the report lists a surviving mutant with that function's file and line and the change it made

### Requirement: Mutation testing runs on demand over one path or the whole scope

A developer SHALL be able to run the same mutation testing over one source path of the workspace, the crates outside the full check included, with the stage's report and without touching the baseline; and over the whole scope of the full check (the core layers and copy-pool) to rewrite the floors from the result.

#### Scenario: Mutation testing of one path on demand

- **GIVEN** a developer who names one source path of the workspace
- **WHEN** they run the mutation recipe for it
- **THEN** only the mutants of that path run, against the tests of its crate
- **AND** the recipe prints the same report as the stage, without changing the baseline

#### Scenario: The floors are rewritten from a whole run

- **GIVEN** a developer who runs the baseline recipe
- **WHEN** it ends
- **THEN** every mutant of the core layers and copy-pool ran and the coverage was measured
- **AND** the baseline holds the new floors and the commit they came from

### Requirement: Dynamic analysis runs on demand for the unsafe-free core and the native adapters

A single recipe SHALL run the unit tests of the application's core under an interpreter that detects undefined behavior and the integration tests of the main-queue, focus and text-pasting adapters under an address sanitizer, on a nightly toolchain pinned to one date for that recipe alone; the stable toolchain the project builds with SHALL stay the one every other recipe uses. A detected undefined behavior or memory error SHALL fail the recipe and name the test and the finding.

#### Scenario: Undefined behavior in a core test fails the recipe

- **GIVEN** a core unit test that reads uninitialized memory
- **WHEN** the dynamic analysis recipe runs
- **THEN** it fails and names the test and the undefined behavior

#### Scenario: A memory error in a native adapter fails the recipe

- **GIVEN** a native adapter integration test that reads past the end of a buffer
- **WHEN** the dynamic analysis recipe runs
- **THEN** it fails and names the test and the memory error

#### Scenario: The project still builds on stable

- **GIVEN** the pinned nightly toolchain installed for the dynamic analysis recipe
- **WHEN** `just test` runs
- **THEN** it builds and runs on the project's stable toolchain, as before
