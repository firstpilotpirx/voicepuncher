## ADDED Requirements

### Requirement: A copy of a speech recognition service is ready once it has transcribed speech
The service scripts and the app SHALL count a copy of a speech recognition service they start as ready only after it answered the transcription of a short utterance, and SHALL report the copy ready together with how long that first transcription took, so the first request a client sends a copy never waits for the copy's first decode. A copy the app starts again after it stopped SHALL be warmed the same way before it counts ready; a copy the scripts' supervisor starts again after a crash is warmed by its first request instead. The checks of a running copy SHALL send it no speech.

#### Scenario: The service scripts report a copy ready after its first transcription
- **GIVEN** no local service runs on the chosen ports
- **WHEN** the service scripts start the speech recognition service for commands
- **THEN** each copy is reported ready with the time of its first transcription, and the copy's log shows that transcription before the report

#### Scenario: The app counts a started copy ready after its first transcription
- **GIVEN** the app started a copy of a speech recognition service
- **WHEN** the copy answers on its port but has not transcribed the utterance the app sent it yet
- **THEN** the app does not count the copy ready until the copy transcribed that utterance

#### Scenario: A copy that cannot transcribe in time is not ready
- **GIVEN** the app started a copy of a speech recognition service that answers on its port
- **WHEN** the copy does not answer the transcription of the utterance within the time a start may take
- **THEN** the copy counts as not ready in time and is started again, as a copy that never answered

#### Scenario: The check of a ready copy sends it no speech
- **GIVEN** a copy the app started is ready
- **WHEN** the app checks the copy while it runs
- **THEN** the check asks only whether the copy answers, and sends it no speech to transcribe
