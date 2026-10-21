## ADDED Requirements

### Requirement: A pipeline that fails stops in order and starts again
The runtime SHALL keep the process running when the dictation pipeline ends with an error or a panic: the pipeline's recording stops, the overlay hides, the failure is logged, and the runtime starts a new pipeline from the same configuration and reports Ready; when the new pipeline fails the same way within 30 seconds of its start, the runtime SHALL report the state Failed with the reason and start no other pipeline until a reload.

#### Scenario: The pipeline panics once
- **GIVEN** the runtime is Ready
- **WHEN** the dictation pipeline panics during a recording
- **THEN** the failure is logged and the overlay is hidden
- **AND** the runtime starts a new pipeline and is Ready again, and the hotkey starts a recording

#### Scenario: The restarted pipeline panics again at once
- **GIVEN** every pipeline the runtime starts panics on its first recording
- **WHEN** the user starts a recording twice
- **THEN** the runtime state is Failed with a reason naming the panic
- **AND** the process still runs
