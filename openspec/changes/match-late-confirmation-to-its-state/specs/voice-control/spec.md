## MODIFIED Requirements

### Requirement: A command is carried out once however fast its confirmation comes back
The command spotting service SHALL carry out every command said exactly once, as the command it was said as, whether the speech recognition service for commands answers its confirmations at once or slowly within its time limit; a check that falls due while another is still waiting SHALL wait its turn and hear the same audio it would have heard on time, never be skipped. A confirmation SHALL be read against the commands that were listened for when its words were heard, even when the app switched to the commands of another state before the confirmation came back; a command the app's current state does not accept is then ignored, as any command said in the wrong state is.

#### Scenario: A command is carried out once while confirmations come back slowly
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **WHEN** the user says the doubled short form "restart restart" in the middle of dictated speech
- **THEN** the dictation is restarted exactly once

#### Scenario: A chain is carried out the same at any confirmation speed
- **GIVEN** listening is on and a dictation runs
- **WHEN** the user says "stop enter start" once while confirmations come back at once and once while each takes 2 seconds
- **THEN** both times the dictation ends, its text is pasted, Return is pressed and a new dictation starts, each exactly once

#### Scenario: A start confirmed after the dictation began stays a start
- **GIVEN** listening is on and no dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **AND** the app switches to the commands of a running dictation as soon as a dictation starts
- **WHEN** the user says "start dictation" followed by dictated speech
- **THEN** a dictation starts exactly once
- **AND** the dictation is not restarted
