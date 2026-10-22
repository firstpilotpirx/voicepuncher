# Spec Delta

## MODIFIED Requirements

### Requirement: Refinement failure falls back to the raw transcript
The system SHALL paste the raw transcript when the text refinement service fails, times out, or returns text that is cut off or less than half as long as the transcript; it SHALL do so the same way while the machine is busy with other work, so the outcome never depends on load.

#### Scenario: Service stops between recordings
- **WHEN** the user finishes a recording after the text refinement service has stopped
- **THEN** the raw transcript is pasted and the failure is logged

#### Scenario: Service hits its output limit
- **WHEN** the service stops generating because it reached its output limit
- **THEN** the raw transcript is pasted instead of the cut-off text

#### Scenario: Service hits its output limit while the machine is under load
- **GIVEN** the service stops generating at its output limit
- **AND** other recognition and voice control checks run on the machine at the same time
- **WHEN** the user transcribes the same recording 20 times in a row
- **THEN** every time the service receives exactly one request and the raw transcript is pasted instead of the cut-off text

#### Scenario: Missing refinement request is told apart from a lost transcription
- **GIVEN** the service stops generating at its output limit
- **WHEN** a transcription with refinement ends without a request reaching the service
- **THEN** the failure report names what went wrong first — the app's own exit status and log, or every connection the service dropped and why — so a lost transcription is never reported as a missing request
