# Spec Delta

## ADDED Requirements

### Requirement: Push-to-talk works with voice control on or off
The system SHALL keep voice control off unless the config file or a command-line flag turns it on. Whether voice control is on or off, the hotkey SHALL record as push-to-talk does: a press starts a dictation, a press in toggle mode or the release in hold mode ends it and pastes its text, and Escape cancels it.

#### Scenario: The hotkey records when voice control is off
- **GIVEN** voice control is not turned on
- **WHEN** the user presses the hotkey
- **THEN** a recording starts

#### Scenario: The hotkey records when voice control is on
- **GIVEN** voice control is turned on
- **AND** no dictation runs
- **WHEN** the user presses the hotkey
- **THEN** a dictation starts
- **AND** the start sound plays
- **AND** the overlay shows the recording

#### Scenario: The hotkey ends its dictation when voice control is on
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user presses the hotkey again
- **THEN** the sentence is pasted into the focused app
- **AND** listening stays on

#### Scenario: Escape cancels a dictation when voice control is on
- **GIVEN** voice control is turned on
- **AND** a dictation runs
- **WHEN** the user presses Escape
- **THEN** nothing is pasted
- **AND** the cancel sound plays
- **AND** the overlay hides
- **AND** listening stays on


### Requirement: The hotkey ends a dictation started by voice
With voice control on, a hotkey press in toggle mode, or its release in hold mode, during a dictation SHALL end the dictation as "stop dictation" does and paste its text, with the stop sound, whether the hotkey or "start dictation" started it. Listening SHALL stay on.

#### Scenario: A hotkey press ends a voice dictation and keeps listening
- **GIVEN** the user started a dictation by saying "start dictation"
- **AND** the user has said a sentence in it
- **WHEN** the user presses the hotkey
- **THEN** the stop sound plays
- **AND** the sentence is pasted into the focused app
- **AND** listening stays on

### Requirement: Voice commands end a dictation the hotkey started
With voice control on, the system SHALL listen for the dictation phrases during a dictation the hotkey started, so "stop dictation" ends it and pastes its text without the command, and "cancel dictation" discards it.

#### Scenario: Saying stop dictation ends a hotkey dictation
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user says "stop dictation"
- **THEN** the sentence is pasted into the focused app without "stop dictation"

#### Scenario: Saying cancel dictation discards a hotkey dictation
- **GIVEN** voice control is turned on
- **AND** the user started a dictation with the hotkey and said a sentence
- **WHEN** the user says "cancel dictation"
- **THEN** nothing is pasted
- **AND** the cancel sound plays
- **AND** listening stays on

### Requirement: Listening stays on while voice control is on
With voice control on, the system SHALL start listening as soon as the app starts, without a hotkey press, and SHALL keep listening until the app exits or the command spotting service stops answering. The hotkey SHALL NOT turn listening off. Turning listening on SHALL play its sound.

#### Scenario: Listening is on at start
- **GIVEN** voice control is turned on
- **WHEN** the app starts
- **THEN** listening is on
- **AND** the listening-on sound plays

#### Scenario: Listening stays on after a hotkey dictation
- **GIVEN** listening is on
- **WHEN** the user records a dictation with the hotkey
- **THEN** listening is on
- **AND** saying "press enter" presses Return

### Requirement: Each command but cancel has a one-word short form
The system SHALL take "enter" as "press enter", "clear" as "clear all text", "start" as "start dictation" and "stop" as "stop dictation", with or without a trailing "please", under the same rules as the full phrase: said on its own, in the state that accepts the command. "cancel dictation" has no short form.

#### Scenario: A short form is recognized as its command
- **GIVEN** listening is on
- **WHEN** the user says "enter", "clear", "start" or "stop" on its own, in the state that accepts its command
- **THEN** "press enter", "clear all text", "start dictation" or "stop dictation" takes effect

## MODIFIED Requirements

### Requirement: The user's own way of saying the commands is recognized
The system SHALL recognize the commands as the user says them, including small slips of pronunciation ("star dictation", "presenter"). Every command the user said in a recording kept for testing SHALL be recognized exactly once, apart from recordings marked unclear; repeats said without a half-second pause are one command. A short Russian phrase SHALL NOT be taken for a command; "стоп" is left out of this rule, since the speech recognition service may hear it as the short form "stop".

#### Scenario: Every recorded command of the user is recognized
- **GIVEN** the user's recordings of the commands, each labelled with the command said
- **WHEN** the recordings are heard one after another while listening, the app switching phrase sets as it does
- **THEN** each command said, repeats without a half-second pause counted as one, is recognized exactly once
- **AND** no other command takes effect

#### Scenario: A short Russian phrase is no command
- **GIVEN** voice control is listening
- **WHEN** someone says short Russian phrases ("да", "спасибо", "привет"), each on its own with a pause around it
- **THEN** no command takes effect

### Requirement: "start dictation" starts a dictation
While listening and no dictation runs, the system SHALL start a dictation when the user says "start dictation", exactly as a hotkey press starts a recording in push-to-talk, with the same start sound and overlay. Ending it by voice or by hotkey SHALL play the stop sound and show the processing overlay, and cancelling it SHALL play the cancel sound and hide the overlay, as in push-to-talk.

#### Scenario: Saying start dictation starts a dictation
- **GIVEN** listening is on
- **AND** no dictation runs
- **WHEN** the user says "start dictation"
- **THEN** a dictation starts
- **AND** the start sound plays
- **AND** the overlay shows the recording

#### Scenario: Saying stop dictation plays the stop sound and shows processing
- **GIVEN** the user started a dictation by voice and said a sentence
- **WHEN** the user says "stop dictation"
- **THEN** the stop sound plays
- **AND** the overlay shows processing until the sentence is pasted, then hides

## REMOVED Requirements

### Requirement: Voice control is opt-in
**Reason**: Replaced by "Push-to-talk works with voice control on or off": the hotkey records with voice control on too.
**Migration**: None; voice control stays opt-in.

### Requirement: The hotkey during a dictation ends it and turns listening off
**Reason**: Replaced by "The hotkey ends a dictation started by voice": listening stays on.
**Migration**: To stop listening, quit the app or run without voice control.


### Requirement: The hotkey turns listening on and off
**Reason**: The hotkey records as in push-to-talk again; listening stays on while voice control is on.
**Migration**: To dictate without voice commands, run without voice control.
