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
      And the enter sound plays
      And the overlay shows the enter sign for about a second, then hides

  Rule: "clear all text" erases the focused text

    Scenario: Saying clear all text selects all and deletes
      Given listening is on
      And no dictation runs
      When the user says "clear all text"
      Then select all and delete are pressed in the focused app
      And the clear sound plays
      And the overlay shows the clear sign for about a second, then hides

  Rule: "paste text" pastes the clipboard

    Scenario: Saying paste text pastes the clipboard at the cursor
      Given listening is on
      And no dictation runs
      And the clipboard holds a sentence
      When the user says "paste text"
      Then Cmd+V is pressed in the focused app
      And the paste sound plays
      And the overlay shows the paste sign for about a second, then hides

    Scenario: Paste leaves the clipboard as it was
      Given listening is on
      And no dictation runs
      And the clipboard holds a sentence
      When the user says "paste"
      Then the clipboard still holds the same sentence

  Rule: A command counts on its own or in its set places

    Scenario: A stop phrase inside a dictated sentence does not end the dictation
      Given a dictation runs
      When the user says "please stop dictation software from crashing" without pausing
      Then the dictation keeps running

  Rule: "press enter" during a dictation pastes and sends

    Scenario: Saying enter during a dictation pastes and sends
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "enter"
      Then the sentence is pasted into the focused app
      And the pasted text does not contain "enter"
      And the Return key is pressed after it
      And the enter sound plays and the overlay shows the enter sign after the paste

  Rule: Each state accepts its own commands

    Scenario: Clear during a dictation is ignored
      Given a dictation runs
      When the user says "clear all text"
      Then no key is pressed
      And the dictation keeps running

    Scenario: Paste during a dictation is ignored
      Given a dictation runs
      When the user says "paste text"
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

  Rule: "restart dictation" starts the dictation over

    Scenario: Saying restart drops the dictation and starts a new one
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "restart"
      Then no text is pasted
      And the restart sound plays, and neither the cancel sound nor the start sound
      And the overlay shows the restart sign, then the recording
      And a new dictation starts

    Scenario: Only the speech after a restart is pasted
      Given the user said a sentence, "restart", and another sentence
      When the user says "stop"
      Then only the other sentence is pasted

  Rule: A command may end an utterance

    Scenario: Stop right after the last dictated word
      Given a dictation runs
      When the user says a sentence and then "stop" without a pause, then pauses
      Then the sentence is pasted into the focused app
      And the pasted text does not contain "stop"

  Rule: "start" may open an utterance

    Scenario: Start followed by the dictation at once
      Given listening is on
      And no dictation runs
      When the user says "start" and a sentence without a pause, and later ends the dictation
      Then the sentence is pasted into the focused app
      And the pasted text does not contain "start"

  Rule: A command may end with "please"

    Scenario: Please after start dictation is not pasted
      Given listening is on
      And no dictation runs
      When the user says "start dictation please" and a sentence without a pause, the command recognized before "please" was heard, and later ends the dictation
      Then the sentence is pasted into the focused app
      And the pasted text does not contain "please"

  Rule: A dictation started by voice holds the speech right after the command

    Scenario: Start start followed by the dictation at once
      Given listening is on
      And no dictation runs
      When the user says "start start" and a sentence without a pause, and later ends the dictation
      Then the whole sentence, its first words included, is pasted into the focused app
      And the pasted text does not contain "start"

    Scenario: Start, a short pause, then the dictation
      Given listening is on
      And no dictation runs
      When the user says "start", pauses for about 0.5 s and says a sentence, and later ends the dictation
      Then the whole sentence, its first words included, is pasted into the focused app
      And the pasted text does not contain "start"

    Scenario: Restart, a short pause, then the new dictation
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "restart", pauses for about 0.5 s and says another sentence, and later ends the dictation
      Then only the other sentence is pasted, its first words included

    Scenario: A chain ending in start followed by the next dictation at once
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "stop enter start" and another sentence without a pause, and later ends the dictation
      Then the other sentence, its first words included, is pasted after the Return key is pressed

  Rule: A doubled short form counts anywhere

    Scenario: Stop stop in the middle of speech
      Given a dictation runs
      When the user says a sentence, "stop stop" and more words, without any pause
      Then the sentence is pasted into the focused app
      And the pasted text does not contain "stop"
      And the pasted text does not contain the words after it

    Scenario: Cancel cancel in the middle of speech
      Given a dictation runs
      When the user says a sentence and "cancel cancel" without a pause
      Then no text is pasted
      And the cancel sound plays

  Rule: A chain of commands is carried out in order

    Scenario: Stop enter pastes and sends
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "stop enter"
      Then the sentence is pasted into the focused app
      And the Return key is pressed after it

    Scenario: Stop enter start sends and starts the next dictation
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "stop enter start"
      Then the sentence is pasted into the focused app
      And the Return key is pressed after it
      And a new dictation starts after it

    Scenario: Cancel clear discards and clears
      Given a dictation runs
      When the user says "cancel clear"
      Then no text is pasted
      And select all and delete are pressed in the focused app

  Rule: Which chains exist

    Scenario: Stop clear enter
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "stop clear enter"
      Then the sentence is pasted, then select all and delete are pressed, then the Return key is pressed

    Scenario: Paste enter pastes the clipboard and sends
      Given listening is on
      And no dictation runs
      When the user says "paste enter"
      Then Cmd+V is pressed, then the Return key is pressed

  Rule: Every confirmed command sounds different

    Scenario: Enter and clear sound different by default
      Given listening is on
      And no dictation runs
      When the user says "enter", and later "clear"
      Then the two commands play different sounds

    Scenario Outline: Paste sounds different from enter and clear by default
      Given listening is on
      And no dictation runs
      When the user says "paste", and later "<other>"
      Then the two commands play different sounds

      Examples:
        | other |
        | enter |
        | clear |

  Rule: A confirmation sign gives way to what comes next

    Scenario: Enter start shows the recording right after the enter sign
      Given listening is on
      And no dictation runs
      When the user says "enter start"
      Then the enter sound plays and the overlay shows the enter sign
      And the overlay shows the recording as soon as the new dictation starts

    Scenario: Stop clear enter ends on the enter sign
      Given a dictation runs
      And the user has said a sentence in it
      When the user says "stop clear enter"
      Then the stop sound, the clear sound and the enter sound play in this order
      And the overlay shows the clear sign, then the enter sign, then hides

  Rule: Push-to-talk keeps its sounds and overlay

    Scenario: A hotkey dictation shows no command sign
      Given listening is on
      And no dictation runs
      When the user records a dictation with the hotkey
      Then the start and stop sounds play
      And the overlay shows the recording, then processing, then the Pasted sign, then hides, with no command sign

  Rule: The app keeps reaching for the command spotting service while it is away

    Scenario: The app keeps reaching for the command spotting service when it goes away
      Given listening is on
      When the command spotting service stops answering
      Then an error naming the command spotting service is logged
      And no sound plays
      And listening stays on

  Rule: Listening comes back when the command spotting service is back

    Scenario: Commands work again after the command spotting service restarts
      Given listening is on
      And the command spotting service went away and is back
      When the user says "press enter"
      Then the Return key is pressed in the focused app
      And no listening sound played after the app started

  Rule: The overlay shows that voice commands do not work while the service stays away

    Scenario: A short outage shows nothing
      Given listening is on
      And no dictation runs
      When the command spotting service goes away and is back within 5 seconds
      Then the overlay never shows that voice commands do not work
      And no sound plays

    Scenario: A long outage in standby shows the sign until the service is back
      Given listening is on
      And no dictation runs
      And the overlay is hidden
      When the command spotting service stays away for longer than 5 seconds, then is back
      Then the overlay shows that voice commands do not work while the service is away
      And once the service is back, the overlay hides

    Scenario: A long outage during a dictation shows the sign beside the recording sign
      Given a dictation runs
      And the overlay shows the recording sign
      When the command spotting service stays away for longer than 5 seconds, then is back
      Then the overlay shows that voice commands do not work beside the recording sign
      And once the service is back, the overlay shows the recording sign alone

    Scenario: A dictation that ends during a long outage keeps the sign
      Given a dictation runs
      And the user has said a sentence in it
      And the overlay shows that voice commands do not work
      When the user ends the dictation with the hotkey and its text is pasted, and later the service is back
      Then the overlay still shows that voice commands do not work after the paste
      And once the service is back, the overlay hides

  Rule: A command whose confirmation fails is reported

    Scenario: The app shows a failed confirmation
      Given listening is on
      When the command spotting service reports a failed confirmation
      Then the overlay shows the failure notice that voice commands are unavailable

  Rule: A dictation keeps recording while the command spotting service is away

    Scenario: The hotkey ends a dictation while the service is away
      Given a dictation runs
      And the user has said a sentence in it
      And the command spotting service went away during it
      When the user presses the hotkey once
      Then the sentence is pasted into the focused app
      And nothing was pasted before the dictation was ended

    Scenario: "stop dictation" ends a dictation after the service is back
      Given a dictation runs
      And the user has said a sentence in it
      And the command spotting service went away during it and is back
      When the user says "stop dictation"
      Then the sentence is pasted into the focused app
      And nothing was pasted before the dictation was ended

  Rule: A command said while the command spotting service was away still counts

    Scenario: "stop dictation" said during a short outage ends the dictation
      Given a dictation runs
      And the user has said a sentence in it
      And the command spotting service went away during it
      When the service is back and reports "stop dictation", said while it was away
      Then the sentence is pasted into the focused app
      And the pasted text does not contain the stop phrase

  Rule: Every voice start logs how long its start cue took

    Scenario: A voice start logs its timing
      Given listening is on
      And no dictation runs
      When the user says "start start" and a sentence without a pause
      Then the log shows one timing line for that start, with its total and the time of each stage

    Scenario: The timing stays right after microphone audio was lost
      Given listening is on
      And some seconds of microphone audio never reached the command spotting service earlier
      When the user says "start" and stays silent
      Then the logged total matches the time from the end of "start" to the start cue within 0.1 s

    Scenario: A hotkey start logs no voice start timing
      Given listening is on
      And no dictation runs
      When the user presses the hotkey once
      Then the log shows no voice start timing line
