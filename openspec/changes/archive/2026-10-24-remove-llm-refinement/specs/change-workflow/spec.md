## MODIFIED Requirements

### Requirement: A test whose precondition is missing is skipped and counted, never green

A test that needs a service, a model, recordings or the desktop SHALL never pass by returning early when that need is missing. The recipe that runs it SHALL leave it out when the need is missing and SHALL print, before the tests run, how many tests it left out and which need was missing. A test the recipe runs SHALL fail, naming the missing need, when that need is missing after all.

#### Scenario: The unit run leaves out the tests that need the services and says so

- **GIVEN** no speech recognition or command spotting service is running for the run
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
