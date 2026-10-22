## ADDED Requirements

### Requirement: A pipeline that fails stops in order and starts again
The runtime SHALL keep the process running when the dictation pipeline ends with an error or a panic: the pipeline's recording stops, the overlay hides, the failure is logged and a new pipeline starts from the same configuration; a new pipeline that fails within 30 seconds of that start SHALL leave the runtime Failed with the reason and no pipeline.

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
