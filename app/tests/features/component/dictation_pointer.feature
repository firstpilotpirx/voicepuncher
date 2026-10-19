# The pipeline runs on its own thread with stand-ins: the recorder hands over one second of
# audio, speech recognition and text refinement take a moment, a text field has keyboard focus
# unless a scenario says otherwise, and the overlay and the dictation sign record what they
# were asked to show in one ordered log. The dictation sign is driven through the adapter that
# shows it next to the pointer, with a stand-in for its window; the failure notice lasts a
# moment instead of 4 seconds. The steps block while they watch the pipeline, so @serial keeps
# other scenarios from stalling them. Where the sign sits next to the pointer, and that the
# system pointer stays as it is, are the window's business: its adapter test checks them.
@dictation_pointer @component @serial
Feature: dictation-pointer

  Rule: The dictation sign shows next to the pointer while a dictation runs

    Scenario: A hotkey recording shows the dictation sign
      Given no dictation runs yet for the dictation sign
      When the user starts a recording with the hotkey
      Then the dictation sign shows a red microphone

    Scenario: A recording started by voice shows the dictation sign
      Given voice control is turned on and no dictation runs for the dictation sign
      When the user says "start dictation" to start a dictation
      Then the dictation sign shows a red microphone

    Scenario: The dictation sign mirrors every stage of a dictation
      Given text refinement is enabled and a button has keyboard focus for the dictation sign
      When the user records a dictation with speech in it, ends it and then clicks into a text field
      Then the dictation sign shows a red microphone, then an ear with a turning ring, then sparkles with a turning ring, then an orange keyboard, then a green check

    Scenario: A failure shows a warning triangle in the dictation sign
      Given the speech recognition service is not running while a recording runs with the dictation sign
      When the user ends the recording with the hotkey while the dictation sign shows
      Then the dictation sign shows a yellow warning triangle until the failure notice hides

    Scenario: A command sign leaves the dictation sign as it was
      Given voice control is turned on, a recording runs and the dictation sign shows
      When the user says "clear all text" during the recording
      Then the dictation sign keeps showing the red microphone

  Rule: The dictation sign goes away once the dictation is over

    Scenario: A pasted dictation removes the dictation sign with the overlay
      Given a recording runs and the dictation sign shows
      When the user ends the recording and the text is pasted
      Then the dictation sign is gone when the overlay hides after the Pasted sign

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

    Scenario: Commands unavailable in standby show no dictation sign
      Given listening is on and no dictation runs for the dictation sign
      When the command spotting service stays away for longer than 5 seconds
      Then the overlay shows the commands-unavailable sign and no dictation sign shows
