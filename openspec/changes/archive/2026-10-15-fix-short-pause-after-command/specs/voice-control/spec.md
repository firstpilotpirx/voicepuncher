## MODIFIED Requirements

### Requirement: An utterance ends after 0.3 s of silence
The system SHALL end an utterance after 0.3 s of silence, so a command said after a short breath is an utterance of its own. This SHALL hold for a quiet voice and when speech follows the pause at once: a command said on its own, followed by 0.3–0.6 s of silence and then speech, SHALL take effect. The speech after the pause SHALL NOT change what the command is heard as. A pause is the silence between the words as said, not the silence a speech synthesizer adds after a word.

#### Scenario: Stop after a short breath
- **GIVEN** a dictation runs
- **WHEN** the user says a sentence, pauses for about 0.35 s and says "stop dictation", then pauses
- **THEN** the sentence is pasted into the focused app without "stop dictation"

#### Scenario: Restart, a short pause, then speech that starts like another word
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart", pauses for about 0.4 s and says another sentence that opens with a hard consonant, and later ends the dictation
- **THEN** only the other sentence is pasted, its first words included

#### Scenario: A command alone, a short pause, then speech
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter", pauses for about 0.3 s and says a sentence, then pauses
- **THEN** the Return key is pressed once
- **AND** no other command takes effect

#### Scenario: The user's recorded command alone, a short pause, then speech
- **GIVEN** the user's own recordings of the commands exist on the machine
- **AND** the state accepts the command
- **WHEN** the user's recorded "restart", "start", "stop" or "enter", said on its own, is followed by 0.3–0.6 s of silence and then speech
- **THEN** the command takes effect once
- **AND** no other command takes effect
