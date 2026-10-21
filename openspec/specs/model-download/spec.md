# model-download Specification

## Purpose
The app downloads the model files its configuration needs into the models folder — in the Download window at first launch, or headless with `--download-models` — from pinned addresses, resumable and checked against pinned sha256 values.

## Requirements

### Requirement: Missing models are listed before anything downloads
When the app manages its services and a model file its configuration needs is missing, it SHALL open the Download window before any service starts. The window SHALL list every model file the configuration needs, each with its name, what it is for, its size and its status (Present or Missing), the total size of the missing files, the free space of the models folder, and SHALL download nothing until the user chooses Download.

#### Scenario: A launch with the dictation model missing
- **GIVEN** the app manages its services and the dictation model is not in the models folder
- **WHEN** the app launches
- **THEN** the Download window lists the dictation model as Missing with its size and purpose
- **AND** the files that are present are listed as Present
- **AND** nothing is downloaded and no service is started

#### Scenario: A launch with every model present shows no window
- **GIVEN** every model file the configuration needs is in the models folder
- **WHEN** the app launches
- **THEN** no Download window opens and the services start

### Requirement: A model is present by its place and size
A model file SHALL count as present when it is in the models folder, in `~/.voicepuncher/models` or in `~/.voicepuncher` with the size the catalog gives, or when the variable that names it names an existing file; the keyword spotting folder SHALL count as present when the folder exists. Presence SHALL NOT read the files' contents.

#### Scenario: A file of the wrong size is missing
- **GIVEN** the dictation model file in the models folder is shorter than the catalog size
- **WHEN** the app checks the models
- **THEN** the dictation model is Missing

#### Scenario: A model named by its variable counts as present
- **GIVEN** the variable for the dictation model names an existing file outside the models folder
- **WHEN** the app checks the models
- **THEN** the dictation model is Present

### Requirement: Download fetches, verifies and places each file
Choosing Download SHALL fetch every missing file, one at a time, from its pinned address to a temporary file `NAME.part` in the models folder, computing its sha256 while it streams, and SHALL move it to its name only when the sha256 matches the catalog. The keyword spotting archive SHALL be extracted into the models folder and then deleted. The window SHALL show each file's progress in percent and the total received out of the total needed.

#### Scenario: Every missing file is downloaded and placed
- **GIVEN** two needed files are missing
- **WHEN** the user chooses Download
- **THEN** each file is fetched in turn with its progress shown, its checksum verified, and moved to its name in the models folder
- **AND** no `.part` file is left

#### Scenario: The keyword spotting archive is extracted
- **GIVEN** the keyword spotting folder is missing
- **WHEN** its archive is downloaded and verified
- **THEN** the folder is in the models folder and the archive is gone

### Requirement: A download resumes where it stopped
A cancelled or interrupted download SHALL keep its `.part` file, and the next Download SHALL continue from the bytes already received, asking the server only for the rest; a `.part` larger than the catalog size SHALL be deleted and the file fetched from the start, and a server that answers a resume with the whole file SHALL be read from the start.

#### Scenario: Cancel and download again
- **GIVEN** a file was half downloaded when the user chose Cancel
- **WHEN** the user chooses Download again
- **THEN** only the remaining bytes are fetched
- **AND** the placed file's checksum matches the catalog

#### Scenario: A server that ignores the resume
- **GIVEN** a `.part` of a file exists and the server answers the resume with the whole file
- **WHEN** the file is downloaded
- **THEN** the file is read from its start and placed with a matching checksum

### Requirement: A file whose checksum does not match is downloaded again
A file whose sha256 does not match the catalog SHALL have its `.part` deleted and be downloaded once more from the start; a second mismatch SHALL fail that file with `checksum mismatch` and go on with the next file.

#### Scenario: A corrupted part is downloaded again
- **GIVEN** a `.part` whose bytes were changed
- **WHEN** the file is downloaded
- **THEN** the mismatch is found, the file is fetched again from the start and placed with a matching checksum

#### Scenario: A file that never matches is reported
- **GIVEN** the server sends bytes that never match the catalog checksum
- **WHEN** the file is downloaded
- **THEN** the file is fetched twice and then reported Failed with checksum mismatch
- **AND** nothing is placed under its name

### Requirement: Without internet the user is told and nothing is deleted
A connection failure SHALL be retried 3 times, after 2, 4 and 8 seconds; after the last try the file SHALL be reported Failed with `no internet connection`, its `.part` and every present file SHALL be kept, and the download SHALL go on with the next file.

#### Scenario: No connection
- **GIVEN** no connection can be made to the model hosts
- **WHEN** the user chooses Download
- **THEN** each missing file is tried 4 times with pauses of 2, 4 and 8 seconds and then reported Failed with no internet connection
- **AND** the files that were present are still present

### Requirement: Not enough space is refused before starting
When the free space of the models folder is smaller than the bytes still missing, Download SHALL fetch nothing and the window SHALL name the amount needed and the amount free.

#### Scenario: A disk that is too full
- **GIVEN** 3 GB are missing and 1 GB is free
- **WHEN** the user chooses Download
- **THEN** nothing is fetched
- **AND** the window reads that 3 GB are needed and 1 GB is free

### Requirement: The services start when the last file is present
When every needed file is present, the runtime SHALL leave Waiting for models and start the services, without a relaunch; the window's Done button SHALL be enabled then, and closes the window.

#### Scenario: The last file arrives
- **GIVEN** the runtime waits for models
- **WHEN** the last missing file is placed
- **THEN** the runtime starts the services and becomes Ready once they answer
- **AND** Done is enabled in the window

### Requirement: The same download runs headless
`voicepuncher --download-models [--models-dir DIR] [--no-llm]` SHALL run the same download without a window, for every catalog file (without the text refinement model with `--no-llm`), printing one line per file event and per 10 % of a file's progress to the error output, and SHALL exit 0 when every file is present at the end and 1 otherwise; `just models` SHALL run it.

#### Scenario: A headless run with every file present
- **GIVEN** a folder that holds every catalog file
- **WHEN** `voicepuncher --download-models --models-dir` names that folder
- **THEN** nothing is fetched and every file is reported present
- **AND** the command exits 0

#### Scenario: A headless download that fails
- **GIVEN** a folder that misses one catalog file and a host for it that refuses connections
- **WHEN** `voicepuncher --download-models --models-dir` names that folder
- **THEN** the command names the failed file and exits 1
