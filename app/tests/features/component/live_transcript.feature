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

  # Drawn on the real overlay on the main thread.
  Rule: The shown text keeps the overlay at a bounded size

    Scenario: A long text shows its latest words
      Given the overlay is ready
      When a text longer than the overlay can show arrives during the recording
      Then the text window shows the latest words after an ellipsis and the pill keeps its size

  # On the microphone recorder's buffer, whose fill decides the warning.
  Rule: The recording-limit warning appears only at the limit

    Scenario: A short live recording logs no limit warning
      Given a live recording shorter than the recording limit
      When its last chunk cuts the recording at its end and the recording stops
      Then the recording does not count as cut off at its limit

    Scenario: A recording that reached the limit still warns
      Given a recording whose audio fills the recording limit
      When the recording stops
      Then the recording counts as cut off at its limit
