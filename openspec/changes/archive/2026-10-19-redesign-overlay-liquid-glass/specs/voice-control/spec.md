# Spec Delta

## MODIFIED Requirements

### Requirement: Push-to-talk keeps its sounds and overlay
A recording started and ended by the hotkey SHALL play the sounds and show the overlay it shows today — the recording, then processing, then the Pasted sign — with no command sign; Escape during it SHALL play the cancel sound and hide the overlay.

#### Scenario: A hotkey dictation shows no command sign
- **GIVEN** listening is on
- **WHEN** the user records a sentence with the hotkey
- **THEN** the start and stop sounds play
- **AND** the overlay shows the recording, then processing, then the Pasted sign, then hides, with no command sign
