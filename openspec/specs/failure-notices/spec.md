# failure-notices Specification

## Purpose
Tells the user on screen, right away, what failed whenever an action they started (a dictation, a voice command, voice control) cannot be completed, so a failure is never mistaken for "it did not hear me".

## Requirements

### Requirement: A failed action shows a failure notice
The system SHALL show a failure notice in the overlay when an action the user started cannot be completed because a local service, a device or a permission failed. The notice SHALL name the failing part in plain words (speech recognition, text refinement, command spotting, microphone, accessibility) and, when a remedy is known, say it. It SHALL appear within 2 s of the failure, take no keyboard focus, and the log SHALL keep the technical detail.

#### Scenario: The notice names the failing service and its remedy
- **GIVEN** the speech recognition service is not running
- **WHEN** the user records a dictation with the hotkey and stops it
- **THEN** within 2 s of the stop the overlay shows a notice that names speech recognition and says to start the local services
- **AND** the app the user dictates into keeps the keyboard focus

### Requirement: A lost dictation is announced
The system SHALL show a failure notice saying the dictation was lost, and why, when the transcript of a dictation cannot be produced, whether the dictation was started by the hotkey or by voice.

#### Scenario: A hotkey dictation is lost
- **GIVEN** the speech recognition service does not answer
- **WHEN** the user stops a dictation started with the hotkey
- **THEN** nothing is pasted and the overlay shows that the dictation was lost because speech recognition failed

#### Scenario: A voice dictation is lost
- **GIVEN** voice control is on and the speech recognition service does not answer
- **WHEN** the user says "stop dictation" after a dictation started by voice
- **THEN** nothing is pasted and the overlay shows that the dictation was lost because speech recognition failed

### Requirement: A partly lost dictation is announced
The system SHALL show a failure notice saying part of the dictation was lost when one or more chunks of a long dictation cannot be transcribed and the rest of it is pasted.

#### Scenario: One chunk of a long dictation is lost
- **GIVEN** the speech recognition service fails on one chunk of a long dictation and answers on the others
- **WHEN** the user stops the dictation
- **THEN** the text of the other chunks is pasted and the overlay shows that part of the dictation was lost

### Requirement: Text pasted without refinement is announced
The system SHALL show a failure notice saying the text was pasted without refinement when text refinement fails or its answer is rejected and the raw transcript is pasted instead. No notice is shown when the user turned refinement off.

#### Scenario: The text refinement service is down
- **GIVEN** text refinement is on and the text refinement service is not running
- **WHEN** the user stops a dictation
- **THEN** the raw transcript is pasted and the overlay shows that the text was pasted without refinement

#### Scenario: Refinement turned off shows no notice
- **GIVEN** the user turned text refinement off
- **WHEN** the user stops a dictation
- **THEN** the raw transcript is pasted and no failure notice is shown

### Requirement: A failed paste is announced
The system SHALL show a failure notice saying the text was not pasted when pasting a transcript fails, naming the accessibility permission when it is missing.

#### Scenario: Pasting is refused
- **GIVEN** pasting into the focused app fails
- **WHEN** the user stops a dictation that was transcribed
- **THEN** the overlay shows that the text was not pasted

### Requirement: A microphone that does not start is announced
The system SHALL show a failure notice naming the microphone when a recording cannot start, whether it was started by the hotkey or by voice.

#### Scenario: The microphone cannot start
- **GIVEN** the microphone cannot be opened
- **WHEN** the user presses the hotkey to start a dictation
- **THEN** no recording runs and the overlay shows that the microphone is unavailable

### Requirement: A voice command that fails is announced
The system SHALL show a failure notice naming the voice command that could not be carried out, and why, when a recognized voice command fails; voice control keeps listening for the next command.

#### Scenario: A key press is refused
- **GIVEN** voice control is on and pressing keys in the focused app fails
- **WHEN** the user says "press enter"
- **THEN** the overlay shows that "press enter" failed
- **AND** the next voice command is still carried out

### Requirement: Voice control that cannot start listening is announced
The system SHALL show a failure notice naming command spotting, and saying to start the local services, when voice control cannot start listening because the command spotting service is unreachable.

#### Scenario: The command spotting service is not running at start
- **GIVEN** voice control is on and the command spotting service is not running
- **WHEN** the app starts
- **THEN** the overlay shows that voice commands are unavailable because command spotting is not running

### Requirement: A failure notice goes away on its own
The system SHALL hide a failure notice on its own a few seconds after it appears, and SHALL let the user start the next dictation while it is shown; a new recording replaces the notice.

#### Scenario: The notice hides by itself
- **GIVEN** a failure notice is shown
- **WHEN** a few seconds pass with no other action
- **THEN** the overlay hides

#### Scenario: A new dictation replaces the notice
- **GIVEN** a failure notice is shown
- **WHEN** the user presses the hotkey to start a dictation
- **THEN** the recording starts and the overlay shows the recording instead of the notice

### Requirement: A repeated failure does not stack notices
The system SHALL keep a single notice on screen when the same failure happens again while its notice is shown, restarting the time it stays, instead of showing a second notice.

#### Scenario: The same failure twice in a row
- **GIVEN** a notice that speech recognition failed is shown
- **WHEN** the next dictation fails the same way before the notice hides
- **THEN** one notice that speech recognition failed stays on screen

### Requirement: A microphone lost during a dictation is announced
The system SHALL show a failure notice naming the microphone when the input device fails or disappears while a dictation runs, whether it was started by the hotkey or by voice.

#### Scenario: The microphone fails during a hotkey dictation
- **GIVEN** a hotkey dictation is in progress
- **WHEN** the input device fails
- **THEN** the overlay shows that the microphone failed and the dictation is over

#### Scenario: The microphone fails during a voice dictation
- **GIVEN** a dictation started by voice is in progress
- **WHEN** the input device fails
- **THEN** the overlay shows that the microphone failed and the dictation is over
