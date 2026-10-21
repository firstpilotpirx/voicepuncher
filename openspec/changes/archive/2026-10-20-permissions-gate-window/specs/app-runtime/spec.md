# Spec Delta

## ADDED Requirements

### Requirement: The runtime waits for permissions
Before it starts the pipeline, the runtime SHALL ask the permissions gate. With a permission missing it SHALL report the state Waiting for permissions, naming the missing permissions, start no pipeline, and leave that state for Starting once every permission is granted. A permission revoked while Ready SHALL stop the pipeline as a stop does and bring the state back to Waiting for permissions. Observers SHALL be told this state like every other.

#### Scenario: Start with a permission missing
- **GIVEN** an observer registered before the runtime starts
- **AND** Input Monitoring is not granted
- **WHEN** the runtime starts
- **THEN** the observer is told Waiting for permissions naming Input Monitoring
- **AND** no dictation pipeline runs

#### Scenario: The gate opens while waiting
- **GIVEN** the runtime is Waiting for permissions
- **WHEN** the last missing permission is granted
- **THEN** the observer is told Starting and then Ready
- **AND** the dictation pipeline runs

#### Scenario: A revocation while Ready
- **GIVEN** the runtime is Ready
- **WHEN** Accessibility is revoked
- **THEN** the pipeline ends
- **AND** the observer is told Waiting for permissions naming Accessibility
