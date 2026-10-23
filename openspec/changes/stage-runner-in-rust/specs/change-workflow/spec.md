## ADDED Requirements

### Requirement: The stage runner is a tested program

The machinery behind the stage recipes (stage keys and reuse, the journal and its summary, lock phases, test ports, frozen tests, named runs, engines staging, the local services) SHALL live in one tested Rust program of the workspace; the `justfile` SHALL only name the stages and call it, hold no Perl and stay under 800 lines. The stage table, the journal line and the pass records SHALL keep their format.

#### Scenario: The same inputs give the keys the former runner gave

- **GIVEN** a scratch repository and environment whose stage keys the former runner computed
- **WHEN** the program computes the keys of the same stages
- **THEN** every key is the one the former runner computed

#### Scenario: A run's journal line keeps its format

- **GIVEN** the report of a run with a passed, a reused, a skipped and a failed stage, a lock wait and hold and a failed test in its output
- **WHEN** the program appends the run to the stage journal
- **THEN** the line holds the fields, their order and their number format the former runner wrote

#### Scenario: The summary reads the journal the former runner wrote

- **GIVEN** a stage journal with lines the former runner appended
- **WHEN** the developer runs `just stage-times`
- **THEN** the summary counts those runs, their stages, their lock holds and their failed tests

#### Scenario: The justfile is a thin layer

- **GIVEN** the `justfile` of the repository
- **WHEN** the tests run
- **THEN** it holds no Perl recipe and fewer than 800 lines

### Requirement: A missing machine lock names its install command

A recipe that runs under the machine-wide lock SHALL check that the lock tool is installed before it starts, and when it is missing SHALL fail naming the install command and the variable that runs the recipes without the lock, instead of failing on a missing file.

#### Scenario: A fresh clone without the lock tool gets the install command

- **GIVEN** a clone on a machine without the agent-kit lock tool and without `VOICEPUNCHER_NO_MACHINE_LOCK=1`
- **WHEN** the developer runs `just check`
- **THEN** it fails before its first stage
- **AND** the message names the command that installs the lock tool and `VOICEPUNCHER_NO_MACHINE_LOCK=1`

### Requirement: A helper's failure surfaces unless it is tolerated on purpose

A recipe of the `justfile` SHALL NOT discard a command's failure with `|| true` unless a comment beside it states which failure is tolerated and why; the tests SHALL fail on an uncommented one.

#### Scenario: An uncommented tolerated failure fails the tests

- **GIVEN** a recipe line that ends in `|| true` with no comment on it or on the line above
- **WHEN** the tests run
- **THEN** they fail and name that line

## MODIFIED Requirements

### Requirement: Recipes run without the machine lock when told so

With `VOICEPUNCHER_NO_MACHINE_LOCK=1` the locked recipes SHALL run their bodies without the agent-kit lock, and `just test` SHALL leave out only the tests of the lock itself, counting them as skipped for the missing lock.

#### Scenario: Without the machine lock the recipes run directly

- **GIVEN** `VOICEPUNCHER_NO_MACHINE_LOCK=1` and no agent-kit lock on the machine
- **WHEN** `just fmt` runs
- **THEN** it runs `cargo fmt --check` without the lock wrapper
- **AND** `just test` reports the tests of the machine lock skipped and runs every other test

#### Scenario: Model tests skip on a machine without models

- **GIVEN** a machine without the command spotting models, where the `justfile` exports their paths empty
- **WHEN** `just test` runs
- **THEN** the command spotting model tests skip and pass
