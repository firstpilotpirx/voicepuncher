# The pipeline runs on its own thread with stand-ins: the recorder hands over one second of
# audio, speech recognition and text refinement take a moment, no text input has focus so the
# text is held back for a short wait, and the overlay and the dictation sign record what they
# were asked to show in one ordered log. The dictation sign is driven through the adapter that
# shows it next to the pointer, with a stand-in for its window. The steps block while they
# watch the pipeline, so @serial keeps other scenarios from stalling them.
@dictation_pointer @component @serial
Feature: dictation-pointer

  Rule: The dictation sign shows next to the pointer while a recording runs

    Scenario: A hotkey recording shows the dictation sign
      Given no dictation runs yet for the dictation sign
      When the user starts a recording with the hotkey
      Then the dictation sign shows while the recording runs

    Scenario: A recording started by voice shows the dictation sign
      Given voice control is turned on and no dictation runs for the dictation sign
      When the user says "start dictation" to start a dictation
      Then the dictation sign shows while the recording runs

    Scenario: Processing shows no dictation sign
      Given the user records a dictation with speech in it and the dictation sign shows
      When the user ends the recording with the hotkey while the dictation sign shows
      Then the dictation sign is gone while speech recognition, text refinement and the wait for focus run

  Rule: The dictation sign goes away as soon as the recording ends

    Scenario: Stopping a recording with the hotkey removes the dictation sign
      Given a recording runs and the dictation sign shows
      When the user ends the recording with the hotkey while the dictation sign shows
      Then the dictation sign is gone

    Scenario: Stopping a recording by voice removes the dictation sign
      Given voice control is turned on, a recording runs and the dictation sign shows
      When the user says "stop dictation" during the recording
      Then the dictation sign is gone

    Scenario: Cancelling a recording removes the dictation sign
      Given a recording runs and the dictation sign shows
      When the user cancels the dictation
      Then the dictation sign is gone

    Scenario: A recording too short to keep removes the dictation sign
      Given the user holds the hotkey down to record and the dictation sign shows
      When the user releases it before the shortest recording the app keeps
      Then the dictation sign is gone

    Scenario: Restarting a dictation shows the sign for the new recording
      Given voice control is turned on, a recording runs and the dictation sign shows
      When the user says "restart dictation" during the recording
      Then the dictation sign shows next to the pointer for the new recording
