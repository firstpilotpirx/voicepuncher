## ADDED Requirements

### Requirement: Settings say what dictation and voice control wait for
While the runtime waits for permissions, the General tab SHALL show under the voice control switch a line naming each missing permission and saying that dictation and voice control start once it is granted; the line SHALL follow the runtime's state at the moment the window opens: it is absent when the runtime does not wait for permissions.

#### Scenario: Voice control on while Accessibility is missing
- **GIVEN** voice control is switched on
- **AND** the runtime waits for Accessibility
- **WHEN** the user opens the General tab
- **THEN** a line under the voice control switch names Accessibility and says dictation and voice control start once it is granted

#### Scenario: No line once every permission is granted
- **GIVEN** the runtime is Ready
- **WHEN** the user opens the General tab
- **THEN** no waiting line is shown under the voice control switch
