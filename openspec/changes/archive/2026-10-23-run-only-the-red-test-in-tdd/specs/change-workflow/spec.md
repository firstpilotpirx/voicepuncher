## ADDED Requirements

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
