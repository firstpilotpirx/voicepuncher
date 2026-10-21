## ADDED Requirements

### Requirement: A first launch creates the configuration file with the defaults
When the configuration file does not exist at launch, the app SHALL write it from a built-in default file before it reads its configuration, log that it created the file, and go on with those defaults; the default file SHALL set every option the settings window shows to its default value with a comment above it saying what the option means, and SHALL parse to the same configuration as an empty file. An existing configuration file SHALL never be replaced at launch.

#### Scenario: A first launch writes the default file
- **GIVEN** no configuration file exists
- **WHEN** the app starts
- **THEN** the configuration file exists with the default file's text
- **AND** the log says the file was created with the defaults

#### Scenario: The default file holds the defaults
- **WHEN** the default file is read as a configuration
- **THEN** it equals the configuration of an empty file

#### Scenario: The default file names every option
- **WHEN** the options of the default file are compared with the options the configuration accepts
- **THEN** every option the settings window shows is set in the default file, with a comment above it

#### Scenario: An existing file is kept
- **GIVEN** a configuration file the user edited
- **WHEN** the app starts
- **THEN** the file is unchanged
