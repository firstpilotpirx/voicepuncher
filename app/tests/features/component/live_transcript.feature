# The pipeline runs on its own thread with stand-ins: the recorder hands over a part of the
# recording whenever a scenario says one is transcribed and the last part at the stop, the
# speech recognition stand-in answers each part with the next scripted text, and the overlay
# and paster record what they were asked to do. The steps block while they watch the
# pipeline, so @serial keeps other scenarios from stalling them.
@live_transcript @component @serial
Feature: live-transcript

  Rule: The overlay shows the text recognized so far while the user dictates

    Scenario: A hotkey dictation shows its first part once it is transcribed
      Given the user dictates with the hotkey
      When the first part of the recording is transcribed while the user speaks
      Then the overlay shows the text of that part next to the recording sign

    Scenario: The shown text grows with every transcribed part
      Given the overlay shows the text of the first part of a running dictation
      When the next part is transcribed
      Then the overlay shows the text of both parts joined, with their shared words once

    Scenario: A dictation started by voice shows its text so far
      Given voice control is turned on and the user started a dictation by saying "start dictation"
      When the first part of the recording is transcribed while the user speaks
      Then the overlay shows the text of that part next to the recording sign

    Scenario: A short dictation shows the recording sign alone
      Given the user dictates with the hotkey for less than one part
      When the recording runs
      Then the overlay shows the recording sign without any text

    Scenario: A part without speech adds no text
      Given the overlay shows the text of the first part of a running dictation
      When the next part holds no speech and is transcribed
      Then the overlay keeps showing the same text

  Rule: The shown text does not outlive its recording or change the paste

    Scenario: Stopping a dictation clears its text
      Given the overlay shows the text of the first part of a running dictation
      When the user ends the dictation
      Then the overlay shows the speech recognition stage without that text
      And the pasted text is the stitched text of every part
