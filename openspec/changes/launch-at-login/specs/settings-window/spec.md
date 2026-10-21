## MODIFIED Requirements

### Requirement: Every option has a control
Every option the configuration file accepts, except retired ones it only tolerates and the records the app keeps there itself (whether the login question was asked), SHALL appear in the settings window as a control on its tab, showing the value the file sets, or the default value when the file does not set it; service addresses and tuning options SHALL sit under an advanced disclosure on their tab.

#### Scenario: A value from the file is shown
- **GIVEN** the configuration file sets the hotkey to the right Command key
- **WHEN** the user opens the General tab
- **THEN** the hotkey control shows Right Command

#### Scenario: An option the file does not set shows its default
- **GIVEN** the configuration file does not set the minimum press length
- **WHEN** the user opens the General tab
- **THEN** the minimum press control shows the default length

#### Scenario: No option is left without a control
- **WHEN** the set of options the configuration file accepts, without the retired ones and the app's own records, is compared with the controls of the settings window
- **THEN** every option has exactly one control

#### Scenario: The app's own record has no control
- **WHEN** the controls of the settings window are listed
- **THEN** none of them edits whether the login question was asked

#### Scenario: The microphone choice lists the Mac's input devices
- **GIVEN** the Mac has two input devices
- **WHEN** the user opens the microphone choice
- **THEN** it offers the system default and both devices by name
