## ADDED Requirements

### Requirement: Keys pressed in the focused app arrive as a keyboard's
The system SHALL press every key it sends to the focused app — Cmd+V of a paste and the keys of the voice commands — as a keyboard does: each modifier as a key of its own, pressed before the key and released after it, with a pause of at least 10 milliseconds after every key event, so the focused app and the keyboard tools that watch the keys see the press.

#### Scenario: Cmd+V presses Command as a key of its own
- **WHEN** the system presses Cmd+V in the focused app
- **THEN** Command goes down, V goes down, V goes up, then Command goes up
- **AND** each event comes at least 10 milliseconds after the one before

#### Scenario: Return goes down and up with a pause between
- **WHEN** the system presses Return in the focused app
- **THEN** Return goes down, then up at least 10 milliseconds later, with no modifier key
