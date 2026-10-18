# Spec Delta

## MODIFIED Requirements

### Requirement: "press enter" presses Return
While listening and no dictation runs, the system SHALL press the Return key in the focused app when the user says "press enter". It SHALL then play the enter sound and show the enter sign on the overlay.

#### Scenario: Saying press enter presses Return
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "press enter"
- **THEN** the Return key is pressed in the focused app
- **AND** the enter sound plays
- **AND** the overlay shows the enter sign for about a second, then hides

### Requirement: "clear all text" erases the focused text
While listening and no dictation runs, the system SHALL select all and delete in the focused app when the user says "clear all text": it presses Cmd+A, then Delete. It SHALL then play the clear sound and show the clear sign on the overlay.

#### Scenario: Saying clear all text selects all and deletes
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "clear all text"
- **THEN** Cmd+A and then Delete are pressed in the focused app
- **AND** the clear sound plays
- **AND** the overlay shows the clear sign for about a second, then hides

### Requirement: "press enter" during a dictation pastes and sends
During a dictation, the system SHALL take "press enter" or "enter", in any of its forms, as "stop dictation" followed by "press enter": it ends the dictation, pastes its text without the command, and then presses Return, with the enter sound and sign once Return is pressed.

#### Scenario: Saying enter during a dictation pastes and sends
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "enter"
- **THEN** the sentence is pasted into the focused app
- **AND** the Return key is pressed after it
- **AND** the enter sound plays and the overlay shows the enter sign after the paste

### Requirement: "restart dictation" starts the dictation over
During a dictation, the system SHALL discard what was dictated so far, paste nothing, and start a new dictation at once when the user says "restart dictation" or its short form "restart", with or without "please". It plays the restart sound alone, neither the cancel sound nor the start sound, and the overlay shows the restart sign for about a second, then the recording again. "restart restart" counts anywhere, like the other doubled short forms, and "restart" may end an utterance. It is in no chain: it only follows a running dictation, and nothing follows it in one breath.

#### Scenario: Saying restart drops the dictation and starts a new one
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "restart"
- **THEN** nothing is pasted
- **AND** the restart sound plays, and neither the cancel sound nor the start sound
- **AND** the overlay shows the restart sign, then the recording
- **AND** a new dictation starts

#### Scenario: Only the speech after a restart is pasted
- **GIVEN** the user said a sentence, "restart", and another sentence
- **WHEN** the user says "stop"
- **THEN** only the other sentence is pasted

#### Scenario: Restart restart in the middle of speech
- **GIVEN** a dictation runs
- **WHEN** someone says a sentence and "restart restart" without a pause
- **THEN** the dictation starts over

## ADDED Requirements

### Requirement: Every confirmed command sounds different
The enter, clear and restart sounds SHALL differ from each other and from the start, stop, cancel, listening-on and listening-off sounds, unless the user picks the same sound for two of them. Each of the three SHALL be the user's to change, like the other sounds, with a distinct sound by default.

#### Scenario: Enter and clear sound different by default
- **GIVEN** the user has not changed any sound
- **WHEN** the user says "enter", and later "clear"
- **THEN** the two commands play different sounds

#### Scenario: A sound the user picked for a command plays
- **GIVEN** the user picked another sound for "clear all text"
- **WHEN** the user says "clear all text"
- **THEN** the picked sound plays

### Requirement: A confirmation sign gives way to what comes next
The enter, clear and restart signs SHALL each name the command that ran. A sign SHALL give way at once to what the overlay shows next — the recording of a dictation a chain starts, or the sign of the next command in the chain — and after about a second the overlay SHALL return to what it showed before the command: hidden in standby, the recording during a dictation.

#### Scenario: Enter start shows the recording right after the enter sign
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "enter start"
- **THEN** the enter sound plays and the overlay shows the enter sign
- **AND** the overlay shows the recording as soon as the new dictation starts

#### Scenario: Stop clear enter ends on the enter sign
- **GIVEN** a dictation runs
- **AND** the user has said a sentence in it
- **WHEN** the user says "stop clear enter"
- **THEN** the stop sound, the clear sound and the enter sound play in this order
- **AND** the overlay shows the clear sign, then the enter sign, then hides

### Requirement: Push-to-talk keeps its sounds and overlay
A recording started and ended by the hotkey SHALL play the sounds and show the overlay it shows today, with no command sign; Escape during it SHALL play the cancel sound and hide the overlay.

#### Scenario: A hotkey dictation shows no command sign
- **GIVEN** listening is on
- **WHEN** the user records a sentence with the hotkey
- **THEN** the start and stop sounds play
- **AND** the overlay shows the recording, then processing, then hides, with no command sign
