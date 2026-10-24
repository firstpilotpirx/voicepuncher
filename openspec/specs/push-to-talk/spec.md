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

### Requirement: The hotkey listener survives every key press
The hotkey listener SHALL tell keys apart by their key codes and modifier flags only, and SHALL NOT ask the system's input-source service what a key types, so no key press can stop the app on a macOS that allows that service only on the main thread.

#### Scenario: The listener reads key codes only
- **GIVEN** the app is built
- **WHEN** its hotkey listener is inspected
- **THEN** it uses no keyboard library that turns key codes into typed text, and no call to the input-source service

#### Scenario: A modifier hotkey is pressed and released
- **GIVEN** the hotkey is right Option
- **WHEN** right Option goes down and up while left Option stays up
- **THEN** the listener reports a press and then a release

#### Scenario: The other side's modifier is not the hotkey
- **GIVEN** the hotkey is right Option
- **WHEN** left Option goes down and up
- **THEN** the listener reports nothing

### Requirement: Holding the hotkey records and releasing it pastes
In Hold mode, pressing the hotkey SHALL start a recording, and releasing it SHALL end the recording, transcribe it and paste the text into the focused app; a press shorter than the shortest recording the app keeps SHALL record nothing and paste nothing.

#### Scenario: A held hotkey records and a release pastes
- **GIVEN** the app is ready in Hold mode and a text field has keyboard focus
- **WHEN** the user holds the hotkey, speaks a phrase and releases the hotkey
- **THEN** a recording runs while the hotkey is held
- **AND** the phrase is transcribed and pasted into the text field after the release

#### Scenario: A short press records nothing
- **GIVEN** the app is ready in Hold mode
- **WHEN** the user releases the hotkey before the shortest recording the app keeps
- **THEN** nothing is transcribed or pasted
