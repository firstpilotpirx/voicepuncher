## ADDED Requirements

### Requirement: Voice control downloads the streaming models
The catalog SHALL hold the English and the Russian streaming models, each a pinned archive extracted into the models folder. With voice control on, the models needed SHALL be the two streaming models and the voice gate model; the catalog SHALL hold neither the Whisper small model nor the keyword spotting model.

#### Scenario: The streaming recognizer needs the streaming models
- **GIVEN** voice control is on with the streaming recognizer and no voice control model is on disk
- **WHEN** the app checks the models
- **THEN** the English and the Russian streaming models and the voice gate model are listed as Missing
- **AND** neither the Whisper small model nor the keyword spotting model is listed

## MODIFIED Requirements

### Requirement: A model is present by its place and size
A model file SHALL count as present when it is in the models folder, in `~/.voicepuncher/models` or in `~/.voicepuncher` with the size the catalog gives, or when the variable that names it names an existing file; a streaming model folder SHALL count as present when the folder exists. Presence SHALL NOT read the files' contents.

#### Scenario: A file of the wrong size is missing
- **GIVEN** the dictation model file in the models folder is shorter than the catalog size
- **WHEN** the app checks the models
- **THEN** the dictation model is Missing

#### Scenario: A model named by its variable counts as present
- **GIVEN** the variable for the dictation model names an existing file outside the models folder
- **WHEN** the app checks the models
- **THEN** the dictation model is Present

### Requirement: Download fetches, verifies and places each file
Choosing Download SHALL fetch every missing file, one at a time, from its pinned address to a temporary file `NAME.part` in the models folder, computing its sha256 while it streams, and SHALL move it to its name only when the sha256 matches the catalog. A streaming model archive SHALL be extracted into the models folder and then deleted. The window SHALL show each file's progress in percent and the total received out of the total needed.

#### Scenario: Every missing file is downloaded and placed
- **GIVEN** two needed files are missing
- **WHEN** the user chooses Download
- **THEN** each file is fetched in turn with its progress shown, its checksum verified, and moved to its name in the models folder
- **AND** no `.part` file is left

#### Scenario: The keyword spotting archive is extracted
- **GIVEN** the keyword spotting folder is missing
- **WHEN** its archive is downloaded and verified
- **THEN** the folder is in the models folder and the archive is gone

## REMOVED Requirements

### Requirement: Voice control downloads the models of its recognizer
**Reason**: its scenarios of the keyword spotter's confirmation are gone with it.
**Migration**: replaced by "Voice control downloads the streaming models".
