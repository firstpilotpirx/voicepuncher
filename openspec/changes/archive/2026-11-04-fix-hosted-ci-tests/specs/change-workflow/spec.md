# Spec Delta

## MODIFIED Requirements

### Requirement: The desktop tests run in the full check when the permissions allow

The tests that use the real microphone, the Accessibility API and windows on the screen SHALL run as a stage of the full check, alone on the machine. A business scenario that needs the desktop SHALL be marked as such on its rule, so the unit run leaves it out and counts it with the other desktop tests and the stage runs it. When the process lacks a permission they need, the stage SHALL run none of them, name the missing permission, print how many tests it left out, show the stage as skipped rather than passed, and never record it as a pass; the user's manual run then runs them. Tests that press keys in the focused app SHALL never run in the full check.

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

#### Scenario: A business rule marked for the desktop is a desktop test

- **GIVEN** a feature file whose rule is marked as needing the desktop
- **WHEN** the unit and integration tests run on a machine without the desktop
- **THEN** the scenarios of that rule do not run and are counted among the desktop tests left out
- **AND** the desktop stage runs them
