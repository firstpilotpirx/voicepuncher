# push-to-talk Specification

## Purpose

Lets the user control recording with a single hotkey: press to start, release (or press again in toggle mode) to stop, then the text is pasted into the focused app.

## Requirements

### Requirement: The hotkey keeps working after a paste
The system SHALL register every press and release of the hotkey that follows a paste, so the next recording can start right after the previous text was pasted.

#### Scenario: Hotkey press after a paste is registered
- **GIVEN** the user pressed and released the hotkey twice
- **AND** the app pasted text into the focused app
- **WHEN** the user presses and releases the hotkey again
- **THEN** both the press and the release are registered
