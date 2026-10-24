## MODIFIED Requirements

### Requirement: The same download runs headless
`voicepuncher --download-models [--models-dir DIR]` SHALL run the same download without a window, for every catalog file, printing one line per file event and per 10 % of a file's progress to the error output, and SHALL exit 0 when every file is present at the end and 1 otherwise; `just models` SHALL run it.

#### Scenario: A headless run with every file present
- **GIVEN** a folder that holds every catalog file
- **WHEN** `voicepuncher --download-models --models-dir` names that folder
- **THEN** nothing is fetched and every file is reported present
- **AND** the command exits 0

#### Scenario: A headless download that fails
- **GIVEN** a folder that misses one catalog file and a host for it that refuses connections
- **WHEN** `voicepuncher --download-models --models-dir` names that folder
- **THEN** the command names the failed file and exits 1
