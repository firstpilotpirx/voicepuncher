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

### Requirement: Nothing is asked in a terminal
The app SHALL ask no question in a terminal, at the first launch or later: the hotkey, the recording mode and every other option come from the configuration file and are changed in the settings window; no command line option starts a question-and-answer setup.

#### Scenario: A first launch asks nothing
- **GIVEN** no configuration file exists
- **WHEN** the app starts with nothing on its standard input
- **THEN** it creates the configuration file and goes on without reading its standard input

#### Scenario: The setup option is gone
- **WHEN** the user starts the app with `--setup`
- **THEN** the app refuses the unknown option with exit code 2
