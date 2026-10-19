# dictation-archive Specification

## Purpose
TBD - created by archiving change dictation-archive. Update Purpose after archive.

## Requirements

### Requirement: Every dictation is kept in an archive entry of its own
The system SHALL keep every dictation that reached speech recognition in an archive entry of its own: one folder per dictation inside one archive folder, named by the recording's start so entries sort by time, holding the dictation's 16 kHz audio, the text of each chunk, the stitched raw transcript and the pasted text, and nothing of another dictation. Its text and metadata SHALL be one versioned machine-readable file. Entries SHALL never leave the machine.

#### Scenario: A pasted dictation is archived with its audio and texts
- **GIVEN** the archive is on
- **WHEN** the user dictates a phrase with the hotkey and it is pasted after text refinement
- **THEN** a new archive entry holds the audio of the dictation, the raw transcript speech recognition returned, and the refined text that was pasted

#### Scenario: A long dictation keeps the text of every chunk
- **GIVEN** the archive is on
- **WHEN** the user dictates long enough for the dictation to be transcribed in several chunks
- **THEN** its archive entry holds the text of each chunk in the order spoken, the stitched raw transcript and the audio of the whole dictation

#### Scenario: Two dictations get two entries
- **GIVEN** the archive is on
- **WHEN** the user dictates two phrases one after the other
- **THEN** the archive holds two entries, each with only its own audio and texts, the later one sorting after the earlier one

### Requirement: An entry says how text refinement went
The system SHALL record in every entry whether the pasted text is the refinement service's answer, the raw transcript because refinement was turned off, or the raw transcript because refinement failed or its answer was rejected.

#### Scenario: Refinement turned off
- **GIVEN** the archive is on and text refinement is turned off
- **WHEN** the user dictates a phrase
- **THEN** its entry says refinement was off and the pasted text equals the raw transcript

#### Scenario: Refinement fails
- **GIVEN** the archive is on and the text refinement service fails
- **WHEN** the user dictates a phrase
- **THEN** its entry says refinement fell back and the pasted text equals the raw transcript

### Requirement: Dictations that paste nothing are archived with the reason
The system SHALL archive a dictation that reached speech recognition but pasted nothing, with its audio and the reason: speech recognition failed, speech recognition returned no text, only command words were transcribed, or the paste failed. A recording the user cancelled, one dropped by "restart dictation", and one shorter than the minimum press time SHALL leave no entry.

#### Scenario: Speech recognition fails
- **GIVEN** the archive is on and the speech recognition service fails
- **WHEN** the user dictates a phrase
- **THEN** its entry holds the audio and says speech recognition failed, with no pasted text

#### Scenario: Only command words were transcribed
- **GIVEN** the archive is on and voice control is on
- **WHEN** the user starts a dictation by voice and stops it without saying anything else
- **THEN** its entry holds the audio and says only command words were transcribed

#### Scenario: A cancelled dictation leaves no entry
- **GIVEN** the archive is on
- **WHEN** the user cancels a dictation
- **THEN** the archive holds no entry for it

#### Scenario: A dictation dropped by restarting leaves no entry
- **GIVEN** the archive is on and voice control is on
- **WHEN** the user starts a dictation by voice, says "restart dictation", dictates a phrase and stops it by voice
- **THEN** the archive holds one entry, with the audio of the restarted dictation only

#### Scenario: A press shorter than the minimum press time leaves no entry
- **GIVEN** the archive is on
- **WHEN** the user records for less than the minimum press time
- **THEN** the archive holds no entry for it

### Requirement: An entry carries what is needed to compare runs
The system SHALL record in every entry the moment the recording started, how it started (hotkey held, hotkey toggled, voice start, or recording file), the audio duration, the time of each stage after the recording stopped, the dictation and text refinement models, and the app version.

#### Scenario: A voice-started dictation is marked as such
- **GIVEN** the archive is on and voice control is on
- **WHEN** the user starts a dictation by voice, dictates a phrase and stops it by voice
- **THEN** its entry says it started by voice and names the app version and the models

### Requirement: The archive is on for dictations and opt-in for recording files
The system SHALL archive live dictations by default, into a folder under the app's home folder, and SHALL let the user turn the archive off or name another folder in the configuration or on the command line. A recording file SHALL be archived only when an archive folder is named on the command line, so test and benchmark runs never write into the user's archive.

#### Scenario: Archive turned off
- **GIVEN** the user turned the archive off
- **WHEN** the user dictates a phrase
- **THEN** no archive entry is written

#### Scenario: A recording file is archived into a named folder
- **GIVEN** an archive folder named on the command line
- **WHEN** the user transcribes a recording file
- **THEN** that folder holds one entry with the file's audio, the raw transcript and the printed text, marked as started from a recording file

#### Scenario: A recording file without a named folder is not archived
- **GIVEN** no archive folder named on the command line
- **WHEN** the user transcribes a recording file
- **THEN** no archive entry is written

### Requirement: The archive stays within its size limit
The system SHALL keep the archive within a size limit the user can set, 2 GB by default: after writing an entry, it SHALL remove the oldest entries until the archive fits, never the entry just written.

#### Scenario: The oldest entries go first
- **GIVEN** an archive whose entries already fill its size limit
- **WHEN** a new entry is written
- **THEN** the oldest entries are removed until the archive fits, and the newest entries, the new one included, stay

### Requirement: Archiving never costs a dictation
The system SHALL write the entry after the paste, and a failure to write or prune the archive SHALL be logged as a warning without changing what was pasted or stopping the app.

#### Scenario: The archive folder cannot be written
- **GIVEN** an archive folder the app cannot write to
- **WHEN** the user dictates a phrase
- **THEN** the phrase is pasted as usual and the log warns that the dictation was not archived
