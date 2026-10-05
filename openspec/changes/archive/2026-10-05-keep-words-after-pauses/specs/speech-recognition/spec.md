## ADDED Requirements

### Requirement: Speech after a pause is transcribed
The system SHALL transcribe the speech that follows a pause in a long dictation or recording file: no phrase of more than five consecutive spoken words SHALL be missing from the raw transcript, whatever the pauses between words or sentences and wherever the chunk seams fall. Words the service mishears are not dropped phrases.

#### Scenario: Speech with pauses between words or sentences
- **GIVEN** a recording file of Russian or English speech longer than the chunk threshold, with pauses between words or between sentences
- **WHEN** the user transcribes the recording file
- **THEN** no run of more than five consecutive spoken words is missing from the printed text

#### Scenario: Speech after pauses wherever the seams fall
- **GIVEN** Russian or English speech longer than the chunk threshold, spoken at different rates or with pauses
- **AND** the speech starts after different lead-ins, so the chunk seams fall on different words
- **WHEN** the speech is transcribed in chunks
- **THEN** no run of more than five consecutive spoken words is missing from the chunk transcripts

### Requirement: No speech yields no text
The system SHALL treat an empty transcript from the speech recognition service as a recording without speech: nothing is pasted or printed, and the cycle ends without a service failure.

#### Scenario: Service hears no speech
- **GIVEN** the speech recognition service returns no text for a recording
- **WHEN** the user transcribes the recording file without refinement
- **THEN** no text is produced and the app does not report a failure
