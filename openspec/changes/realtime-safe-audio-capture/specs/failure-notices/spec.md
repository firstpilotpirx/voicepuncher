# Spec Delta

## ADDED Requirements

### Requirement: A microphone lost during a dictation is announced
The system SHALL show a failure notice naming the microphone when the input device fails or disappears while a dictation runs, whether it was started by the hotkey or by voice.

#### Scenario: The microphone fails during a hotkey dictation
- **GIVEN** a hotkey dictation is in progress
- **WHEN** the input device fails
- **THEN** the overlay shows that the microphone failed and the dictation is over

#### Scenario: The microphone fails during a voice dictation
- **GIVEN** a dictation started by voice is in progress
- **WHEN** the input device fails
- **THEN** the overlay shows that the microphone failed and the dictation is over
