# The pipeline runs on its own thread with stand-ins: the recorder hands over one second of
# audio (and, for a dictation transcribed while the user speaks, a part during the recording
# and the last part at the stop), the speech recognition stand-in takes a moment and notes
# when it starts and finishes, and the overlay, paster and focus probe record
# what they were asked to do. The Pasted sign takes itself down after half a second, so a
# scenario checks that the overlay ends a pasted dictation on it. The steps block while they
# watch the pipeline, so @serial keeps other scenarios from stalling them.
@processing_stages @component @serial
Feature: processing-stages

  Rule: The overlay shows speech recognition until the transcript is ready

    Scenario: A hotkey dictation shows speech recognition after the stop
      Given the user records a dictation with the hotkey
      When the user ends the recording with the hotkey
      Then the overlay shows the speech recognition stage from the stop until the transcript is ready

    Scenario: A dictation ended by voice shows speech recognition after the stop
      Given voice control is turned on and a dictation runs
      When the user says "stop dictation" to end it
      Then the overlay shows the speech recognition stage from the stop until the transcript is ready

    Scenario: A dictation transcribed while the user speaks shows speech recognition for its last part
      Given the user records a dictation transcribed in parts while the user speaks
      When the user ends the recording with the hotkey
      Then the overlay shows the speech recognition stage from the stop until the last part is transcribed

    Scenario: A dictation without speech hides the overlay after speech recognition
      Given the user records a dictation with no speech in it
      When the user ends the recording with the hotkey
      Then the overlay shows the speech recognition stage, then hides

  Rule: The transcript is pasted right after speech recognition

    Scenario: Speech recognition is followed by the paste
      Given the user records a dictation with the hotkey
      When the user ends a dictation with speech in it
      Then the overlay shows the speech recognition stage from the stop until the transcript is ready
      And no other processing stage is shown
      And the overlay shows the Pasted sign once the text is pasted, then hides

    Scenario: Text held back for focus shows the wait after speech recognition
      Given no text input has focus
      When the user ends a dictation with speech in it
      Then the overlay shows the speech recognition stage, then the wait for focus

  Rule: Speech recognition shows no text window

    Scenario: Speech recognition shows no transcript
      Given the user records a dictation with the hotkey
      When the user ends a dictation with speech in it
      Then the overlay shows the speech recognition stage with no text window under it
