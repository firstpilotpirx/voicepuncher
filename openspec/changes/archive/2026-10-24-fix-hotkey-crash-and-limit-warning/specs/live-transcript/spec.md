## ADDED Requirements

### Requirement: The recording-limit warning appears only at the limit
The system SHALL warn that a recording was cut off at its limit only when the captured audio reached that limit; a recording ended earlier, also one whose end was cut by the live chunks, SHALL log no such warning.

#### Scenario: A short live recording logs no limit warning
- **GIVEN** a live recording of a few seconds
- **WHEN** its last chunk cuts the recording at its end and the recording stops
- **THEN** no recording-limit warning is logged

#### Scenario: A recording that reached the limit still warns
- **GIVEN** a recording whose audio filled the recording limit
- **WHEN** the recording stops
- **THEN** the recording-limit warning is logged
