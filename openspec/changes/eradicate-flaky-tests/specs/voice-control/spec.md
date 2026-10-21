## MODIFIED Requirements

### Requirement: A command is carried out once however fast its confirmation comes back
The command spotting service SHALL carry out every command said exactly once, as the command it was said as, whether the speech recognition service for commands answers its confirmations at once or slowly within its time limit; a check that falls due while another is still waiting SHALL wait its turn and hear the same audio it would have heard on time, never be skipped. A confirmation SHALL be read against the commands that were listened for when its words were heard together with those listened for when it comes back, the closest command winning, so a switch to the commands of another state before the confirmation comes back never turns the command said into another one; a command the app's current state does not accept is then ignored, as any command said in the wrong state is. Which commands are carried out SHALL depend only on what was said and on how late each confirmation comes back measured against the speech, never on how busy the machine is while it listens.

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
- **AND** a dictation is started by the hotkey right after the user said "start dictation", before its confirmation comes back
- **WHEN** the confirmation of "start dictation" comes back
- **THEN** it is taken as "start dictation"
- **AND** the dictation is not restarted

#### Scenario: A start said right after a slowly confirmed stop stays a start
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2.5 seconds to answer each confirmation
- **WHEN** the user says "stop dictation" and then, before its confirmation comes back, "start"
- **THEN** the dictation ends and a new dictation starts

#### Scenario: A chain is carried out the same on a busy machine
- **GIVEN** listening is on and a dictation runs
- **AND** the speech recognition service for commands takes 2 seconds to answer each confirmation
- **AND** the machine is busy with other work, so the speech is taken in late and in bursts
- **WHEN** the user says "stop enter start"
- **THEN** the dictation ends, its text is pasted, Return is pressed and a new dictation starts, each exactly once, as on an idle machine
