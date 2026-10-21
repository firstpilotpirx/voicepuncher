## ADDED Requirements

### Requirement: Nothing is asked in a terminal
The app SHALL ask no question in a terminal, at the first launch or later: the hotkey, the recording mode and every other option come from the configuration file and are changed in the settings window; no command line option starts a question-and-answer setup.

#### Scenario: A first launch asks nothing
- **GIVEN** no configuration file exists
- **WHEN** the app starts with nothing on its standard input
- **THEN** it creates the configuration file and goes on without reading its standard input

#### Scenario: The setup option is gone
- **WHEN** the user starts the app with `--setup`
- **THEN** the app refuses the unknown option with exit code 2
