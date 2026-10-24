## ADDED Requirements

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
