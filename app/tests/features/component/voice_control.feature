# The pipeline runs on its own thread with stand-ins: the command spotting service is
# scripted (each spoken phrase becomes the speech and phrase events the service reports),
# the recorder notes what was said and where the recording was cut, the speech recognition
# service answers what was said before the cut, and the paster, key presser and sound player
# record what they were asked to do. The steps block while they watch the pipeline, so
# @serial keeps other scenarios from stalling them.
@voice_control @component @serial
Feature: voice-control

  Rule: Push-to-talk works with voice control on or off

    Scenario: The hotkey records when voice control is off
      Given voice control is not turned on
      When the user presses the hotkey once
      Then a dictation starts

    Scenario: The hotkey records when voice control is on
      Given listening is on
      And no dictation runs
      When the user presses the hotkey once
      Then a dictation starts
      And the start sound plays
      And the overlay shows the recording

    Scenario: The hotkey ends its dictation when voice control is on
      Given the user started a dictation with the hotkey and said a sentence
      When the user presses the hotkey once
      Then the sentence is pasted into the focused app
      And listening stays on

    Scenario: Escape cancels a dictation when voice control is on
      Given a dictation runs
      And the user has said a sentence in it
      When the user presses Escape
      Then no text is pasted
      And the cancel sound plays
      And the overlay is hidden
      And listening stays on

  Rule: Listening stays on while voice control is on

    Scenario: Listening is on at start
      Given voice control is turned on
      When the app starts
      Then listening is on
      And the listening-on sound plays

    Scenario: Listening stays on after a hotkey dictation
      Given listening is on
      When the user records a dictation with the hotkey
      Then listening is on
      And saying "press enter" presses Return

  Rule: "start dictation" starts a dictation

    Scenario: Saying start dictation starts a dictation
      Given listening is on
      And no dictation runs
      When the user says "start dictation"
      Then a dictation starts
      And the start sound plays
      And the overlay shows the recording

    Scenario: Saying stop dictation plays the stop sound and shows processing
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "stop dictation"
      Then the stop sound plays
      And the overlay shows processing until the sentence is pasted, then hides

  Rule: "stop dictation" ends a dictation and pastes its text

    Scenario: Saying stop dictation pastes the dictated text
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "stop dictation"
      Then the sentence is pasted into the focused app
      And the pasted text does not contain the stop phrase
      And listening stays on

  Rule: "cancel dictation" discards a dictation

    Scenario: Saying cancel dictation pastes nothing
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "cancel dictation"
      Then no text is pasted
      And the cancel sound plays
      And listening stays on

  Rule: "press enter" presses Return

    Scenario: Saying press enter presses Return
      Given listening is on
      And no dictation runs
      When the user says "press enter"
      Then the Return key is pressed in the focused app
      And the command sound plays

  Rule: "clear all text" erases the focused text

    Scenario: Saying clear all text selects all and deletes
      Given listening is on
      And no dictation runs
      When the user says "clear all text"
      Then select all and delete are pressed in the focused app
      And the command sound plays

  Rule: A command counts only when spoken on its own

    Scenario: A stop phrase inside a dictated sentence does not end the dictation
      Given a dictation runs
      When the user says "please stop dictation software from crashing" without pausing
      Then the dictation keeps running

  Rule: Each state accepts only its own commands

    Scenario: Press enter during a dictation is ignored
      Given a dictation runs
      When the user says "press enter"
      Then no key is pressed
      And the dictation keeps running

    Scenario: Stop dictation without a dictation is ignored
      Given listening is on
      And no dictation runs
      When the user says "stop dictation"
      Then no text is pasted
      And no sound plays

  Rule: Commands are carried out in the order spoken

    Scenario: Press enter right after stop dictation follows the paste
      Given a dictation runs
      And the user has said "stop dictation"
      When the user says "press enter" before the text is pasted
      Then the text is pasted first
      And the Return key is pressed after it

  Rule: The hotkey ends a dictation started by voice

    Scenario: A hotkey press ends a voice dictation and keeps listening
      Given a dictation runs
      And the user has said a sentence in it
      When the user presses the hotkey once
      Then the stop sound plays
      And the sentence is pasted into the focused app
      And listening stays on

  Rule: Voice commands end a dictation the hotkey started

    Scenario: Saying stop dictation ends a hotkey dictation
      Given the user started a dictation with the hotkey and said a sentence
      When the user says "stop dictation"
      Then the sentence is pasted into the focused app
      And the pasted text does not contain the stop phrase

    Scenario: Saying cancel dictation discards a hotkey dictation
      Given the user started a dictation with the hotkey and said a sentence
      When the user says "cancel dictation"
      Then no text is pasted
      And the cancel sound plays
      And listening stays on

  Rule: Voice control needs the command spotting service

    Scenario: Listening turns off when the command spotting service goes away
      Given listening is on
      When the command spotting service stops answering
      Then an error naming the command spotting service is logged
      And listening is off
