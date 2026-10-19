## ADDED Requirements

### Requirement: The same dictation audio gives the same text
The system SHALL ask the speech recognition service for one deterministic transcription of dictation audio, without falling back to random sampling when the service is unsure, so the same audio always yields the same text.

#### Scenario: Hard audio transcribed twice gives the same text
- **GIVEN** a recording of speech under high noise
- **WHEN** it is transcribed twice in a row
- **THEN** both transcripts are the same text
