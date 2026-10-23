## ADDED Requirements

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
