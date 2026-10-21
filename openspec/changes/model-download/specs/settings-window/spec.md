## ADDED Requirements

### Requirement: The Services tab holds the models and the services
The settings window SHALL have a Services tab, after Logging, holding the options of the `[services]` section and a Models row reading how many needed model files are present out of how many, with Download models… (which opens the Download window) and Show in Finder. Saving a new models folder SHALL apply as every other option, and the runtime SHALL check the models again in that folder.

#### Scenario: The Services tab shows the models
- **GIVEN** every needed model file is in the models folder
- **WHEN** the user opens the Services tab
- **THEN** it shows the models folder and reads that every model is present
- **AND** it offers Download models… and Show in Finder

#### Scenario: A new models folder is checked
- **GIVEN** the runtime is Ready
- **WHEN** the user picks an empty models folder in the Services tab and saves
- **THEN** the runtime waits for models and the Download window lists the files as Missing

## MODIFIED Requirements

### Requirement: One settings window opens from the menu
Choosing Settings in the menu SHALL show the settings window with its tabs (General, Dictation, Refinement, Sounds, Paste, Archive, Logging, Services) and bring it to the front; choosing Settings again while the window exists SHALL bring the same window to the front instead of opening a second one.

#### Scenario: Settings opens the window
- **GIVEN** the app runs
- **WHEN** the user chooses Settings in the menu
- **THEN** the settings window is in front with the General tab shown

#### Scenario: Settings chosen again reuses the window
- **GIVEN** the settings window is open behind another window
- **WHEN** the user chooses Settings in the menu again
- **THEN** the same settings window comes to the front
- **AND** no second settings window exists
