# The pipeline runs on its own thread with the stand-ins of the text-insertion and
# voice-control scenarios: the focus is scripted, the command spotting service reports the
# phrases a scenario says, and the paster and overlay record what they were asked to do. The
# Pasted sign takes itself down after half a second, so a scenario checks that the overlay ends
# a pasted dictation on it and that nothing but a later signal replaces it. The steps block
# while they watch the pipeline, so @serial keeps other scenarios from stalling them.
@visual_feedback @component @serial
Feature: visual-feedback

  Rule: Pasted text shows the Pasted sign for half a second

    Scenario: A pasted dictation shows the Pasted sign
      Given a text field has keyboard focus
      When a dictation finishes
      Then the overlay shows the Pasted sign with no text window, then hides

    Scenario: Text pasted after a wait for focus shows the Pasted sign
      Given the app holds the text back because a button has focus
      When the user clicks into a text field
      Then the overlay shows the Pasted sign with no text window, then hides

    Scenario: Text left in the clipboard shows no Pasted sign
      Given the app holds the text back because a button has focus
      When the user presses the hotkey
      Then the overlay hides without showing the Pasted sign

    Scenario: A command right after the paste replaces the Pasted sign
      Given voice control is turned on and the overlay shows the Pasted sign
      When the user says "press enter"
      Then the enter sign replaces the Pasted sign at once

  # The rules below draw the real overlay on the main thread and read its first screen.

  Rule: A command sign shows its icon and name in the glass pill

    Scenario: The enter sign shows its icon and name
      Given the overlay is ready
      When the overlay shows the sign of "press enter"
      Then the pill shows the icon "return" and the label "Enter"

  Rule: A failure notice shows an amber glass sign with its message

    Scenario: A lost dictation shows the notice sign with its part's name
      Given the overlay is ready
      When the overlay shows that a dictation was lost to speech recognition
      Then the pill shows the icon "exclamationmark.triangle.fill" and the label "STT"
      And no text window shows

  Rule: Each overlay sign shows an icon and an English label

    Scenario: The processing stage reads in English
      Given the overlay is ready
      When the overlay shows speech recognition
      Then the pill shows the icon "ear" and the label "Recognizing"

    Scenario: The recording shows no label
      Given the overlay is ready
      When a recording starts on the overlay
      Then the pill shows the record sign and a dotted waveform without a label

  Rule: Reduce Motion stops the decorative motion

    Scenario: The record dot pulses only without Reduce Motion, and the waveform still moves
      Given the overlay is ready and the microphone hears speech
      When a recording starts on the overlay and the user speaks
      Then the record dot pulses unless Reduce Motion is on
      And the waveform holds a column taller than a dot

  Rule: The commands-unavailable sign is a small grey glass pill

    Scenario: Standby shows the small CMD pill
      Given the overlay is ready
      When voice commands stop working while no dictation runs
      Then the pill shows the muted microphone with the label "CMD"

    Scenario: A recording shows the muted microphone beside the record sign
      Given the overlay is ready
      When voice commands stop working during a recording
      Then the pill shows the muted microphone beside the record sign

  Rule: The overlay is frosted glass that stays legible over any window

    Scenario: The glass overlay takes no focus and has no bright top line
      Given the overlay is ready
      When a recording starts on the overlay
      Then the glass takes no keyboard focus and has no line along its top edge

  Rule: The pill keeps one size in every state

    Scenario: The pill does not change size from the recording to Pasted
      Given the overlay is ready and the microphone hears speech
      When the overlay shows the recording, speech recognition and the Pasted sign in turn
      Then the pill has one size in all of them

  Rule: The recording shows a pulsing record sign and a slow scrolling level waveform

    Scenario: A new recording starts with a dotted waveform beside the record sign
      Given the overlay is ready
      When a recording starts on the overlay
      Then the pill shows the record sign and a dotted waveform without a label

    Scenario: Speech leaves tall columns
      Given the overlay is ready and the microphone hears speech
      When a recording starts on the overlay and the user speaks
      Then the waveform holds a column taller than a dot

    Scenario: The recording pill is as long as ChatGPT's dictation bar
      Given the overlay is ready
      When a recording starts on the overlay
      Then the pill is 727 points wide

  Rule: The text window shows only during a recording

    Scenario: The text recognized so far shows below the pill
      Given the overlay is ready
      When the text recognized so far arrives during the recording
      Then the text window shows that text whole below the pill

    Scenario: Ending the recording takes the text window away
      Given the overlay is ready
      When the text recognized so far arrives during the recording
      And the recording ends and speech recognition starts
      Then no text window shows

  Rule: The wait for focus shows an orange glass sign with its explanation

    Scenario: Held-back text shows the wait sign
      Given the overlay is ready
      When the overlay shows the wait for focus
      Then the pill shows the icon "keyboard" and the label "Click a text field"
      And no text window shows
