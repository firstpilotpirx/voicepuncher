# The pipeline runs on its own thread with stand-ins: the recorder hands over one second of
# audio (and, for a dictation transcribed while the user speaks, a part during the recording
# and the last part at the stop), the speech recognition and text refinement stand-ins take a
# moment and note when they start and finish, and the overlay, paster and focus probe record
# what they were asked to do. The steps block while they watch the pipeline, so @serial keeps
# other scenarios from stalling them.
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
      And the text refinement stage is not shown

  Rule: The overlay shows text refinement until the corrected text is ready

    Scenario: Text refinement follows speech recognition
      Given text refinement is enabled
      When the user ends a dictation with speech in it
      Then the overlay shows the speech recognition stage, then the text refinement stage until the corrected text is ready
      And the overlay hides once the text is pasted

    Scenario: With text refinement disabled only speech recognition is shown
      Given text refinement is disabled
      When the user ends a dictation with speech in it
      Then the overlay shows the speech recognition stage from the stop until the transcript is ready
      And the text refinement stage is not shown
      And the overlay hides once the text is pasted

    Scenario: Text held back for focus shows the wait after text refinement
      Given no text input has focus
      When the user ends a dictation with speech in it
      Then the overlay shows the speech recognition stage, then the text refinement stage, then the wait for focus
