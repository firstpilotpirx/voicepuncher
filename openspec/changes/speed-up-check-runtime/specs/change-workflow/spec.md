# Spec Delta

## ADDED Requirements

### Requirement: Each model stage of the check has a time budget

Each model stage of `just check` SHALL have a time budget; a model stage past its budget SHALL fail the run, naming its slowest scenarios with their times and the busiest processes beside it. The unit test stage's budget SHALL only warn, as that stage runs beside other sessions' builds.

#### Scenario: A stage within its budget passes

- **GIVEN** a model stage whose budget is two minutes
- **WHEN** the check runs it in one minute
- **THEN** the stage passes and the stage table shows its time against the budget

#### Scenario: A stage past its budget fails naming the slowest scenarios

- **GIVEN** a model stage whose budget is two minutes
- **WHEN** the check runs it in three minutes
- **THEN** the run fails at that stage
- **AND** the failure names the five slowest scenarios with their times and the busiest processes beside the run

#### Scenario: The unit stage past its budget warns and passes

- **GIVEN** the unit test stage whose budget is one minute
- **WHEN** the unit run takes two minutes
- **THEN** the run passes and prints a warning naming the budget and the time

### Requirement: Voice control smoke scenarios not under timing stream faster than real time

A voice control smoke scenario that does not measure a latency SHALL feed its audio as fast as the command spotting service answers, not at the pace of a microphone, so the smoke set ends sooner than its audio lasts; a scenario that measures a start cue or another wall-clock bound SHALL keep real time.

#### Scenario: A hands-free session not under timing ends before its audio would

- **GIVEN** the command spotting service is running
- **WHEN** the synthesized hands-free session runs as a smoke scenario
- **THEN** the app does everything the session said, in order
- **AND** the scenario ends in less time than its audio lasts

### Requirement: The smoke set keeps one scenario per rule of the services and speech capabilities

For the local services and speech recognition capabilities, each rule SHALL have at most one scenario in the smoke set of `just check`; the other scenarios of the rule SHALL run in the full check's regression set. Voice control keeps two or three synthesized smoke scenarios per command.

#### Scenario: A rule with several scenarios runs one in the check

- **GIVEN** a rule of the local services capability with four scenarios
- **WHEN** the smoke set is counted per rule
- **THEN** one of the four runs in the check and the other three in the full check

### Requirement: Benches are journaled

A run of the streaming bench or the dictation bench SHALL be recorded in the stage journal under its own recipe name with the arguments it ran with, its time and its result; it SHALL never count as a pass of any stage and no run SHALL reuse it. The stage summary SHALL show, per branch, the time spent in checks and the time spent in benches apart.

#### Scenario: A streaming bench run is journaled with its arguments

- **GIVEN** a streaming bench over the recorded clips in fast pace
- **WHEN** it ends
- **THEN** the stage journal holds one run named after the bench with those arguments, its time and its result

#### Scenario: A bench run is never a pass

- **GIVEN** a bench run that passed
- **WHEN** the next check runs on the same inputs
- **THEN** it reuses nothing from the bench run

#### Scenario: The summary shows check and bench time per branch apart

- **GIVEN** a journal with two checks and one bench on one branch
- **WHEN** the user summarizes the stage journal
- **THEN** the branch's row shows the time in checks and the time in benches as two numbers

### Requirement: An agent runs only the tests of the area it optimizes

The project SHALL offer a needed-tests map (per area: its tests, the stages its files feed, the bench options that exercise it) and a recipe that runs only the tests of the area owning a given path. The unit run SHALL fail when the map names an area that exists nowhere. A full suite started on a branch whose change has an open task group that is not the last SHALL warn, naming the area recipe.

#### Scenario: The map names the tests of an area

- **GIVEN** a workspace with a command spotting crate and its bench
- **WHEN** the developer prints the needed-tests map
- **THEN** the crate's row names its unit and integration tests, the stages its files feed and the bench options that exercise it

#### Scenario: A run over one area runs only its tests

- **GIVEN** a workspace with several crates
- **WHEN** the developer runs the area recipe with a path inside one crate
- **THEN** only that crate's tests run
- **AND** the run prints the area's stages and bench options

#### Scenario: An area that exists nowhere fails the unit run

- **GIVEN** the map names an area whose path is not in the workspace
- **WHEN** `just test` runs
- **THEN** it fails naming the area

#### Scenario: A full suite on an open task group that is not the last warns

- **GIVEN** a branch of a change whose first of three task groups is still open
- **WHEN** the full unit suite starts
- **THEN** it prints a warning that names the open group and the area recipe, and runs

#### Scenario: A full suite on the last task group runs without a warning

- **GIVEN** a branch of a change whose last task group is the only open one
- **WHEN** the full unit suite starts
- **THEN** it prints no warning

### Requirement: The lint stage scans secrets over the branch's own commits

On a branch, the secret scanners of the lint stage SHALL scan only the commits the branch adds over `main`; on `main` they SHALL scan the whole history.

#### Scenario: A branch scans the commits since main

- **GIVEN** a branch with two commits over main
- **WHEN** the lint stage runs
- **THEN** the secret scanners are given those two commits, not the whole history

#### Scenario: Main scans the whole history

- **GIVEN** a checkout on main
- **WHEN** the lint stage runs
- **THEN** the secret scanners are given every commit

### Requirement: A failed workflow rule is named and rerun alone

The harness of the change workflow scenarios SHALL report one test per rule of its feature, so that a failure names the rule and a rerun of the failed tests runs only that rule's scenarios.

#### Scenario: A failed rule is reported by its name

- **GIVEN** a workflow rule whose one scenario fails
- **WHEN** the unit run ends
- **THEN** the failed test carries the rule's name, not the whole harness

#### Scenario: A rerun covers only the failed rule's scenarios

- **GIVEN** a unit run in which one workflow rule failed
- **WHEN** the developer reruns the failed tests
- **THEN** only that rule's scenarios run

## MODIFIED Requirements

### Requirement: The full check runs only the stages an edit can affect

Each model stage of `just check` and `just full-check` SHALL be fed only by the files whose code it exercises, so that an edit outside them reuses the stage's earlier pass instead of running it. When no earlier pass with the same inputs exists, the stage SHALL run. A file no stage names SHALL feed every stage. The stage runner's own tests and its journal code SHALL feed only the code stages.

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

#### Scenario: A change to the stage runner's tests reuses every model stage

- **GIVEN** `main` passed `just full-check`
- **AND** a branch that changes only the stage runner's tests
- **WHEN** `just full-check` runs on the branch
- **THEN** every model stage shows as reused from `main`'s pass
