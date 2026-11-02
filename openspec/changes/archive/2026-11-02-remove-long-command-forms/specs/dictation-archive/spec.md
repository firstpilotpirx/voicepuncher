## MODIFIED Requirements

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
- **WHEN** the user starts a dictation by voice, says "restart", dictates a phrase and stops it by voice
- **THEN** the archive holds one entry, with the audio of the restarted dictation only

#### Scenario: A press shorter than the minimum press time leaves no entry
- **GIVEN** the archive is on
- **WHEN** the user records for less than the minimum press time
- **THEN** the archive holds no entry for it
