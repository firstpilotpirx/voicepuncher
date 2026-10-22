## ADDED Requirements

### Requirement: Every pull request has a hosted check

A GitHub Actions workflow SHALL run `just fmt`, `just lint` and `just test` on a macOS Apple Silicon runner for every pull request and every push to `main`, with no model, service or machine lock, leaving out only the tests that need the owner's desktop session or voices; any failing step SHALL fail the check. The landing queue's `full-check` SHALL stay the merge gate for build inputs.

#### Scenario: A formatting error turns the check red

- **GIVEN** a pull request with a file `cargo fmt` would change
- **WHEN** the hosted check runs
- **THEN** the check fails at the formatting step

#### Scenario: A failing unit test turns the check red

- **GIVEN** a pull request with a unit test that fails
- **WHEN** the hosted check runs
- **THEN** the check fails at the test step and names the test

#### Scenario: A clean pull request turns the check green

- **GIVEN** a pull request that formats, lints and tests clean
- **WHEN** the hosted check runs with its caches warm
- **THEN** the check passes within 10 minutes

### Requirement: Recipes run without the machine lock when told so

With `VOICEPUNCHER_NO_MACHINE_LOCK=1` the locked recipes SHALL run their bodies without the agent-kit lock, and `just test` SHALL skip only the lock's own self-test, saying so.


#### Scenario: Without the machine lock the recipes run directly

- **GIVEN** `VOICEPUNCHER_NO_MACHINE_LOCK=1` and no agent-kit lock on the machine
- **WHEN** `just fmt` runs
- **THEN** it runs `cargo fmt --check` without the lock wrapper
- **AND** `just test` reports the machine lock self-test skipped and runs every other test

#### Scenario: Model tests skip on a machine without models

- **GIVEN** a machine without the command spotting models, where the `justfile` exports their paths empty
- **WHEN** `just test` runs
- **THEN** the command spotting model tests skip and pass
