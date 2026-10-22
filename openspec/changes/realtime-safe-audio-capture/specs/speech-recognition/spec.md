# Spec Delta

## ADDED Requirements

### Requirement: A recording loses no audio under load
The system SHALL keep every sample the microphone delivers during a recording, however busy the machine is, and SHALL report in the log, once per recording, any audio it had to drop because the recording could not keep up.

#### Scenario: A long dictation on a busy machine is complete
- **GIVEN** the machine is under heavy load
- **WHEN** the user dictates for a minute
- **THEN** the audio sent to speech recognition is as long as the dictation and holds every sample the microphone delivered

#### Scenario: Audio delivered while the recording is held up is kept
- **GIVEN** a recording is in progress
- **WHEN** the part that collects the audio is held up for a moment while the microphone keeps delivering
- **THEN** the audio delivered meanwhile is in the recording in order, with nothing missing

#### Scenario: Audio dropped is reported once
- **GIVEN** a recording is in progress
- **WHEN** the microphone delivers more audio than the recording can hold back for longer than it can keep
- **THEN** the recording goes on and the log reports once, for that recording, how much audio was dropped

### Requirement: The audio sent to speech recognition is exactly what was recorded
The system SHALL send each chunk of a dictation to speech recognition without silence the user did not record and with its last words whole, whatever the microphone's own sample rate.

#### Scenario: The final chunk ends where the dictation ends
- **GIVEN** the microphone records at a rate other than 16 kHz
- **WHEN** the user stops a dictation
- **THEN** the audio of its last chunk ends with the last sound recorded and carries no trailing silence

#### Scenario: The last words before the stop are whole
- **GIVEN** the microphone records at a rate other than 16 kHz
- **WHEN** the user stops a dictation right after its last word
- **THEN** the last word reaches speech recognition whole and is in the transcript

#### Scenario: Chunks of one dictation join without a gap
- **GIVEN** the microphone records at a rate other than 16 kHz and the user dictates longer than the chunk threshold
- **WHEN** the dictation is cut into chunks
- **THEN** the audio of the chunks laid end to end, overlaps removed, is the recording resampled as one piece, with no click, gap or repeated samples at a seam

### Requirement: A microphone lost during a dictation ends it with a notice
The system SHALL end a dictation whose microphone reports an error or disappears, SHALL transcribe and paste what was recorded before, and SHALL announce the microphone failure, never leaving the user with silence.

#### Scenario: The microphone disappears mid-dictation
- **GIVEN** a dictation is in progress
- **WHEN** the input device is unplugged
- **THEN** the dictation ends at once, the speech recorded before is transcribed and pasted, and the overlay shows that the microphone failed

#### Scenario: The microphone reports an error mid-dictation
- **GIVEN** a dictation is in progress
- **WHEN** the input device reports an error
- **THEN** the dictation ends at once, the speech recorded before is transcribed and pasted, and the overlay shows that the microphone failed

#### Scenario: Nothing recorded before the failure
- **GIVEN** a dictation has just started
- **WHEN** the input device reports an error before any speech was recorded
- **THEN** the dictation ends with nothing pasted and the overlay shows that the microphone failed
