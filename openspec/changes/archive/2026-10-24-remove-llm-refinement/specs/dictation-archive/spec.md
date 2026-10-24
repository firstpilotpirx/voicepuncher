## MODIFIED Requirements

### Requirement: Every dictation is kept in an archive entry of its own
The system SHALL keep every dictation that reached speech recognition in an archive entry of its own: one folder per dictation inside one archive folder, named by the recording's start so entries sort by time, holding the dictation's 16 kHz audio, the text of each chunk, the stitched raw transcript and the pasted text, and nothing of another dictation. Its text and metadata SHALL be one versioned machine-readable file. Entries SHALL never leave the machine.

#### Scenario: A pasted dictation is archived with its audio and texts
- **GIVEN** the archive is on
- **WHEN** the user dictates a phrase with the hotkey and it is pasted
- **THEN** a new archive entry holds the audio of the dictation, the raw transcript speech recognition returned, and the text that was pasted

#### Scenario: A long dictation keeps the text of every chunk
- **GIVEN** the archive is on
- **WHEN** the user dictates long enough for the dictation to be transcribed in several chunks
- **THEN** its archive entry holds the text of each chunk in the order spoken, the stitched raw transcript and the audio of the whole dictation

#### Scenario: Two dictations get two entries
- **GIVEN** the archive is on
- **WHEN** the user dictates two phrases one after the other
- **THEN** the archive holds two entries, each with only its own audio and texts, the later one sorting after the earlier one

### Requirement: An entry carries what is needed to compare runs
The system SHALL record in every entry the moment the recording started, how it started (hotkey held, hotkey toggled, voice start, or recording file), the audio duration, the time of each stage after the recording stopped, the dictation model, and the app version. An entry SHALL say nothing about text refinement.

#### Scenario: A voice-started dictation is marked as such
- **GIVEN** the archive is on and voice control is on
- **WHEN** the user starts a dictation by voice, dictates a phrase and stops it by voice
- **THEN** its entry says it started by voice and names the app version and the dictation model

#### Scenario: An entry has no refinement fields
- **GIVEN** the archive is on
- **WHEN** the user dictates a phrase
- **THEN** its entry names no text refinement model and no refinement outcome

## REMOVED Requirements

### Requirement: An entry says how text refinement went

**Reason**: Text refinement is removed; every pasted text is the transcript as speech recognition returned it, without filler sounds.
**Migration**: Entries written before keep their refinement fields; the entry format's version moves up, so a reader tells the two apart.
