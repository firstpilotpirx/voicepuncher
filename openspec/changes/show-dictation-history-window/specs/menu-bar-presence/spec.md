## ADDED Requirements

### Requirement: Dictation history opens from the menu
The menu SHALL offer Dictation history… in every runtime state. Choosing it SHALL bring the history window to the front, opening it when it is closed; choosing it again while the window is open SHALL not open a second one.

#### Scenario: Dictation history opens the history window
- **GIVEN** the runtime is Ready
- **WHEN** the user chooses Dictation history… in the menu
- **THEN** the history window opens in front

#### Scenario: Choosing it again keeps one window
- **GIVEN** the history window is open behind another window
- **WHEN** the user chooses Dictation history… in the menu
- **THEN** the same history window comes to the front and no second one opens
