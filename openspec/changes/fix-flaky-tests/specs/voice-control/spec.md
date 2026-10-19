## ADDED Requirements

### Requirement: A command is carried out once however fast its confirmation comes back
The command spotting service SHALL carry out every command said exactly once, whether the speech recognition service for commands answers its confirmations at once or slowly within its time limit; a check that falls due while another is still waiting SHALL wait its turn and hear the same audio it would have heard on time, never be skipped.

#### Scenario: A command is carried out once while confirmations come back slowly
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **WHEN** the user says "restart dictation"
- **THEN** the dictation is restarted exactly once

#### Scenario: A chain is carried out the same at any confirmation speed
- **GIVEN** listening is on and a dictation runs
- **WHEN** the user says "stop enter start" once while confirmations come back at once and once while each takes 2 seconds
- **THEN** both times the dictation ends, its text is pasted, Return is pressed and a new dictation starts, each exactly once
