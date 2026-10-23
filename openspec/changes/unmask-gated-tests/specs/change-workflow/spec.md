## ADDED Requirements

### Requirement: A test whose precondition is missing is skipped and counted, never green

A test that needs a service, a model, recordings or the desktop SHALL never pass by returning early when that need is missing. The recipe that runs it SHALL leave it out when the need is missing and SHALL print, before the tests run, how many tests it left out and which need was missing. A test the recipe runs SHALL fail, naming the missing need, when that need is missing after all.

#### Scenario: The unit run leaves out the tests that need the services and says so

- **GIVEN** no speech recognition, command spotting or text refinement service is running for the run
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
- **AND** the stage table shows the stage as skipped with the number of tests left out
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
