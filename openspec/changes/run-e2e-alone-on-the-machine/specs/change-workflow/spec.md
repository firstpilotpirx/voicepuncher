# Spec Delta

## ADDED Requirements

### Requirement: E2E stages run alone on the machine

Every model stage of a stage run (`check`, `full-check`, the E2E recipes) SHALL hold the machine-wide test lock exclusively, so no two runs run model stages at once, and the build and test stages of every other run, standalone `just test` included, SHALL not run while it holds it. The stage table and the stage journal SHALL show the wait for that lock and its hold.

#### Scenario: Two runs' model stages take turns

- **GIVEN** a run whose model stages hold the machine lock
- **WHEN** another run reaches its model stages
- **THEN** it waits until the first run returns the lock, and only then starts its model stages

#### Scenario: A build waits while model stages run

- **GIVEN** a run whose model stages hold the machine lock
- **WHEN** another session starts `just test` or a check reaches its build stages
- **THEN** that stage starts only after the model stages returned the lock

#### Scenario: Model stages wait for running builds and keep new ones out

- **GIVEN** a build stage of another run is running
- **WHEN** a run asks the lock for its model stages, and another build stage starts after that
- **THEN** the model stages start only after the running build ended
- **AND** the later build starts only after the model stages returned the lock

#### Scenario: Builds still run beside each other

- **GIVEN** no run holds the lock for its model stages
- **WHEN** two sessions run build or test stages at once
- **THEN** both run at the same time

#### Scenario: The journal shows the lock wait and hold

- **WHEN** a run with model stages ends
- **THEN** its stage table and its journal line show how long it waited for the lock and how long it held it
