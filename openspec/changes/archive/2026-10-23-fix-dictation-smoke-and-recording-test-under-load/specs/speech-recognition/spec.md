# Spec Delta

## ADDED Requirements

### Requirement: A check transcribes one recording file at a time
A check of these requirements that transcribes recording files SHALL run the scenarios that transcribe a file one at a time, so no request waits behind another app's request on the same copy of the speech recognition service and every request is measured against its own deadline. When the service still does not answer a request within its deadline, the check SHALL report the copies that did not answer, the busiest processes on the machine and every check that held a lock at that moment.

#### Scenario: Recording files are transcribed beside another check
- **GIVEN** a voice control check of another checkout runs on the machine
- **WHEN** the smoke scenarios that transcribe recording files run
- **THEN** every scenario gives the result it gives on an idle machine

#### Scenario: A copy that does not answer in time is reported with the load
- **GIVEN** the speech recognition service does not answer a recording file's request within its deadline during a check
- **WHEN** the check reports the failure
- **THEN** the report names the copies that did not answer, the busiest processes on the machine and every check that held a lock at that moment

## MODIFIED Requirements

### Requirement: A recording loses no audio under load
The system SHALL keep every sample the microphone delivers during a recording, however busy the machine is, holding back at least ten seconds of audio while the part that collects it is held up, and SHALL report in the log, once per recording, any audio it had to drop because the recording could not keep up.

#### Scenario: A long dictation on a busy machine is complete
- **GIVEN** the machine is under heavy load
- **WHEN** the user dictates for a minute
- **THEN** the audio sent to speech recognition is as long as the dictation and holds every sample the microphone delivered
- **AND** a check of this scenario that finds samples missing says how much audio was dropped and how long the minute took to deliver

#### Scenario: Audio delivered while the recording is held up is kept
- **GIVEN** a recording is in progress
- **WHEN** the part that collects the audio is held up for several seconds while the microphone keeps delivering
- **THEN** the audio delivered meanwhile is in the recording in order, with nothing missing

#### Scenario: Audio dropped is reported once
- **GIVEN** a recording is in progress
- **WHEN** the microphone delivers more audio than the recording can hold back for longer than it can keep
- **THEN** the recording goes on and the log reports once, for that recording, how much audio was dropped
