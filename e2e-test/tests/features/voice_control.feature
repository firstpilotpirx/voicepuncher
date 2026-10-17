# The phrases are spoken by macOS `say` and streamed to the real command spotting service as
# the app streams the microphone while listening; the app's recognizer decides what counts
# as a command. The hands-free message runs the real pipeline against the real service, with
# stand-ins for the recorder, speech recognition, paste and key presses; sounds and the hotkey
# are covered by the component scenarios and the manual run.
@voice_control @e2e
Feature: voice-control

  Rule: A command counts on its own or in its set places

    @smoke
    Scenario Outline: A command phrase spoken on its own is recognized
      Given the command spotting service is running
      When the user says "<phrase>" with a pause before and after it
      Then the "<phrase>" command takes effect

      Examples:
        | phrase           |
        | start dictation  |
        | stop dictation   |
        | cancel dictation |
        | restart dictation |
        | press enter      |
        | clear all text   |

    @smoke
    Scenario Outline: A command said after a long time of listening is recognized
      Given the command spotting service is running
      And listening has been on for a long time with nothing said
      When the user says "<phrase>" with a pause before and after it
      Then the "<phrase>" command takes effect

      Examples:
        | phrase           |
        | start dictation  |
        | stop dictation   |
        | cancel dictation |
        | restart dictation |
        | press enter      |
        | clear all text   |

    @smoke
    Scenario: A command phrase inside a sentence is ignored while listening
      Given the command spotting service is running
      When someone says "I will press enter when I am done" without pausing
      Then no command takes effect

    @accuracy
    Scenario: Ordinary speech triggers no command
      Given the command spotting service is running
      And a fixed sample of 20 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
      When someone speaks every recording of the sample while listening
      Then no command takes effect

  Rule: The user's own way of saying the commands is recognized

    @smoke @voice_samples
    Scenario Outline: The user's quiet restart after any pause before it
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      When the user's quiet recorded restart follows a dictated sentence after <pause> s of silence while dictating
      Then the commands "restart dictation" take effect in order

      Examples:
        | pause |
        | 0.3   |
        | 0.4   |
        | 0.5   |
        | 0.6   |
        | 0.7   |
        | 0.8   |

    @smoke
    Scenario Outline: A command said in a quiet voice is recognized
      Given the command spotting service is running
      When the user says "<said>" in a quiet voice with a pause before and after it
      Then the "<command>" command takes effect

      Examples:
        | said              | command           |
        | start dictation   | start dictation   |
        | stop dictation    | stop dictation    |
        | cancel dictation  | cancel dictation  |
        | restart dictation | restart dictation |
        | press enter       | press enter       |
        | start             | start dictation   |
        | stop              | stop dictation    |
        | cancel            | cancel dictation  |
        | restart           | restart dictation |
        | enter             | press enter       |
        | clear             | clear all text    |

      # The voice activity detector misses a quiet "clear all text": change detect-quiet-clear-all-text.
      @wip
      Examples:
        | said              | command           |
        | clear all text    | clear all text    |

    @smoke
    Scenario: A short Russian phrase is no command
      Given the command spotting service is running
      When someone says short Russian phrases, each on its own with a pause around it
      Then no command takes effect

  Rule: Every command has a one-word short form

    @smoke
    Scenario Outline: A short form is recognized as its command
      Given the command spotting service is running
      When the user says "<said>" with a pause before and after it
      Then the "<command>" command takes effect

      Examples:
        | said           | command           |
        | start          | start dictation   |
        | start please   | start dictation   |
        | stop           | stop dictation    |
        | stop please    | stop dictation    |
        | cancel         | cancel dictation  |
        | cancel please  | cancel dictation  |
        | restart        | restart dictation |
        | restart please | restart dictation |
        | enter          | press enter       |
        | enter please   | press enter       |
        | clear          | clear all text    |
        | clear please   | clear all text    |

  Rule: A command may end with "please"

    @smoke
    Scenario Outline: A command followed by please is recognized
      Given the command spotting service is running
      When the user says "<said>" with a pause before and after it
      Then the "<command>" command takes effect

      Examples:
        | said                    | command          |
        | start dictation please  | start dictation  |
        | stop dictation please   | stop dictation   |
        | cancel dictation please | cancel dictation |
        | restart dictation please | restart dictation |
        | press enter please      | press enter      |
        | clear all text please   | clear all text   |

  Rule: "clear text" erases the focused text too

    @smoke
    Scenario Outline: Clear text is recognized as clear all text
      Given the command spotting service is running
      When the user says "<said>" with a pause before and after it
      Then the "clear all text" command takes effect

      Examples:
        | said              |
        | clear text        |
        | clear text please |

  Rule: Commands are carried out in the order spoken

    @smoke
    Scenario: A message is dictated and sent by voice alone
      Given the command spotting service is running
      And listening is on
      When the user says "start dictation", a sentence, "stop dictation" and "press enter", each command on its own
      Then the sentence is pasted into the focused app
      And the Return key is pressed after it

  Rule: Each state accepts its own commands

    @smoke
    Scenario Outline: Start or let's start on its own during a dictation is ignored
      Given the command spotting service is running
      When someone says "<words>" on its own after a sentence, <level>, while dictating
      Then no command takes effect

      Examples:
        | words       | level              |
        | start       | at a normal level  |
        | start       | in a quiet voice   |
        | let's start | at a normal level  |
        | let's start | in a quiet voice   |

    @smoke
    Scenario Outline: Start ending a dictated sentence is ignored
      Given the command spotting service is running
      When someone says "<words>" in one breath, <level>, while dictating
      Then no command takes effect

      Examples:
        | words                         | level             |
        | we are all here, let's start  | at a normal level |
        | we are all here, let's start  | in a quiet voice  |
        | the meeting is about to start | at a normal level |
        | the meeting is about to start | in a quiet voice  |

    @smoke
    Scenario Outline: Start inside a dictated sentence is ignored
      Given the command spotting service is running
      When someone says "<words>" in one breath, <level>, while dictating
      Then no command takes effect

      Examples:
        | words                       | level             |
        | let's start with the budget | at a normal level |
        | let's start with the budget | in a quiet voice  |
        | we start at nine tomorrow   | at a normal level |
        | we start at nine tomorrow   | in a quiet voice  |

    @smoke
    Scenario: Stop dictation without a dictation is ignored
      Given the command spotting service is running
      When someone says "stop dictation" in one breath while no dictation runs
      Then no command takes effect

  Rule: A command may end an utterance

    @smoke
    Scenario Outline: A command right after other speech takes effect
      Given the command spotting service is running
      When someone says "<words>" in one breath while <state>
      Then the commands "<commands>" take effect in order

      Examples:
        | words                                    | state             | commands          |
        | We meet on Friday at ten stop            | dictating         | stop dictation    |
        | That is all for today, stop dictation    | dictating         | stop dictation    |
        | Forget it, cancel                        | dictating         | cancel dictation  |
        | Never mind, cancel dictation             | dictating         | cancel dictation  |
        | Let me say it again, restart             | dictating         | restart dictation |
        | Let me try once more, restart dictation  | dictating         | restart dictation |
        | Okay, press enter                        | no dictation runs | press enter       |
        | We meet on Friday at ten, enter          | dictating         | press enter       |
        | Okay, enter                              | no dictation runs | press enter       |
        | Okay, clear                              | no dictation runs | clear all text    |
        | Okay, clear all text                     | no dictation runs | clear all text    |
        | Okay, start                              | no dictation runs | start dictation   |
        | Okay, start dictation                    | no dictation runs | start dictation   |

    @smoke
    Scenario: A command word inside a sentence is still no command
      Given the command spotting service is running
      When someone says "We should stop the server tonight" in one breath while dictating
      Then no command takes effect

  Rule: "start" may open an utterance

    @smoke
    Scenario Outline: Start followed by the dictation at once
      Given the command spotting service is running
      When someone says "<words>" in one breath while no dictation runs
      Then the commands "start dictation" take effect in order

      Examples:
        | words                                         |
        | Start, the meeting moved to Friday at ten     |
        | Start dictation, the build is green again     |

    @smoke
    Scenario: Enter followed by speech is no command
      Given the command spotting service is running
      When someone says "Enter the building on the left" in one breath while no dictation runs
      Then no command takes effect

  # Real-time sessions, like the hands-free ones below: @serial keeps the other scenarios'
  # requests from queuing ahead of their answers.
  @serial
  Rule: A dictation started by voice holds the speech right after the command

    @smoke
    Scenario: Start start followed by the dictation at once
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "Start, start" and a sentence in one breath, and later ends the dictation
      Then the whole sentence, its first words included, is pasted without "start"

    @smoke
    Scenario: Start, a short pause, then the dictation
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "Start", pauses for about 0.5 s and says a sentence, and later ends the dictation
      Then the whole sentence, its first words included, is pasted without "start"

    @smoke
    Scenario: Restart, a short pause, then the new dictation
      Given the command spotting service is running
      And listening is on for a whole session
      And a dictation runs and someone has said a sentence in it
      When someone says "Restart", pauses for about 0.5 s and says another sentence, and later ends the dictation
      Then only the other sentence is pasted, its first words included

    @smoke @voice_samples
    Scenario: The user's recorded restart restart followed by the new dictation at once
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      And a dictation runs and someone has said a sentence in it
      When the user's recorded "restart restart" is followed at once by another sentence, and the dictation later ends
      Then only the other sentence is pasted, its first words included

    @smoke @voice_samples
    Scenario: The user's recorded start start followed by the dictation at once
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      When the user's recorded "start start" is followed at once by a sentence, and the dictation later ends
      Then the whole sentence, its first words included, is pasted without "start"

  Rule: A doubled short form counts anywhere

    @smoke
    Scenario Outline: A doubled short form in the middle of speech takes effect once
      Given the command spotting service is running
      When someone says "<words>" in one breath while <state>
      Then the commands "<commands>" take effect in order

      Examples:
        | words                                                           | state             | commands         |
        | We move the meeting to Friday stop stop and some more words here | dictating         | stop dictation   |
        | Fine, enter enter, thanks                                       | no dictation runs | press enter      |
        | We move the meeting to Friday cancel cancel                     | dictating         | cancel dictation |
        | Wait, restart restart                                           | dictating         | restart dictation |
        | Okay, start start                                               | no dictation runs | start dictation   |
        | Fine, clear clear                                               | no dictation runs | clear all text    |

  Rule: A chain of commands is carried out in order

    @smoke
    Scenario Outline: A chain of commands takes effect in order
      Given the command spotting service is running
      When someone says "<words>" in one breath while <state>
      Then the commands "<commands>" take effect in order

      Examples:
        | words                           | state             | commands                                     |
        | That is all, stop enter         | dictating         | stop dictation, press enter                  |
        | That is all, stop, clear        | dictating         | stop dictation, clear all text               |
        | That is all, stop, enter, start | dictating         | stop dictation, press enter, start dictation |
        | That is all, stop, clear, enter | dictating         | stop dictation, clear all text, press enter  |
        | Forget it, cancel, clear        | dictating         | cancel dictation, clear all text             |
        | Forget it, cancel, enter        | dictating         | cancel dictation, press enter                |
        | That is all, enter, start       | dictating         | press enter, start dictation                 |
        | Okay, enter, clear              | no dictation runs | press enter, clear all text                  |
        | Okay, clear, enter              | no dictation runs | clear all text, press enter                  |
        | Okay, enter, start              | no dictation runs | press enter, start dictation                 |
        | Okay, clear, start              | no dictation runs | clear all text, start dictation              |

  Rule: An utterance ends after 0.3 s of silence

    @smoke @serial
    Scenario: Stop after a short breath
      Given the command spotting service is running
      And listening is on for a whole session
      And a dictation started by voice runs
      When someone says a sentence, takes a short breath and says "stop dictation"
      Then the whole sentence, its first words included, is pasted without "stop"

    @smoke @serial
    Scenario: Restart, a short pause, then speech that starts like another word
      Given the command spotting service is running
      And listening is on for a whole session
      And a dictation runs and someone has said a sentence in it
      When someone says "Restart", pauses for about 0.4 s and says another sentence, and later ends the dictation
      Then only the other sentence is pasted, its first words included

    @smoke
    Scenario: A command alone, a short pause, then speech
      Given the command spotting service is running
      When someone says "Enter", pauses for 0.3 s and says a sentence while no dictation runs
      Then the commands "press enter" take effect in order

    @smoke @voice_samples
    Scenario Outline: The user's recorded command alone, a short pause, then speech
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      When the user's recorded "<said>", said on its own, is followed by <pause> s of silence and a sentence while <state>
      Then the commands "<command>" take effect in order

      Examples:
        | said    | state             | pause | command           |
        | restart | dictating         | 0.3   | restart dictation |
        | restart | dictating         | 0.45  | restart dictation |
        | restart | dictating         | 0.6   | restart dictation |
        | stop    | dictating         | 0.3   | stop dictation    |
        | stop    | dictating         | 0.45  | stop dictation    |
        | stop    | dictating         | 0.6   | stop dictation    |
        | start   | no dictation runs | 0.3   | start dictation   |
        | start   | no dictation runs | 0.45  | start dictation   |
        | start   | no dictation runs | 0.6   | start dictation   |
        | enter   | no dictation runs | 0.3   | press enter       |
        | enter   | no dictation runs | 0.45  | press enter       |
        | enter   | no dictation runs | 0.6   | press enter       |

  # The sessions run in real time and wait for the speech recognition service's answers;
  # @serial keeps the other scenarios' requests from queuing ahead of them.
  @serial
  Rule: A whole hands-free session runs as said

    @smoke
    Scenario: A hands-free session in a synthesized voice
      Given the command spotting service is running
      And listening is on for a whole session
      When a synthesized voice goes through the hands-free session
      Then the app does everything the session said, in order

    @smoke @voice_samples
    Scenario: A hands-free session with the user's recorded commands
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      When the hands-free session is spoken with the user's recorded commands
      Then the app does everything the session said, in order

    @smoke @voice_samples
    Scenario: A hands-free session with long pauses between its steps
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      When the hands-free session is spoken with the user's recorded commands and long pauses
      Then the app does everything the session said, in order

  Rule: Confirmations are not steered by a prompt

    @smoke
    Scenario Outline: Speech that is no command is not turned into a command
      Given the command spotting service is running
      When someone says <speech>, each on its own with a pause around it
      Then no command takes effect

      Examples:
        | speech                 |
        | short Russian phrases  |
        | Russian counting words |

  Rule: Voice control needs the command spotting service to start

    @smoke
    Scenario: The app refuses to start without the command spotting service
      Given voice control is turned on
      And the command spotting service is not running
      When the app starts
      Then the app exits with an error that names the command spotting service and how to start it

  @serial @resilience
  Rule: A slow command spotting service does not stop listening

    Scenario: A confirmation that takes longer than 10 seconds keeps listening on
      Given the speech recognition service for commands and the command spotting service run on ports of their own
      And listening is on with them and a dictation runs
      And the speech recognition service for commands takes 15 seconds to answer one confirmation of the command spotting service
      When the user keeps dictating through those 15 seconds and then says "stop dictation"
      Then listening is still on and no error about the command spotting service is logged
      And the dictation ends and its text is pasted

  @serial @resilience
  Rule: Listening comes back when the command spotting service is back

    Scenario: Commands work again after the command spotting service restarts
      Given the speech recognition service for commands and the command spotting service run on ports of their own
      And listening is on with them
      And the command spotting service went away and was started again
      When the user says "press enter" 5 seconds after the service answers again
      Then the Return key is pressed in the focused app
      And the listening-on sound played when the service came back

  @serial @resilience
  Rule: A dictation keeps recording while the command spotting service is away

    Scenario: "stop dictation" ends a dictation after the service is back
      Given the speech recognition service for commands and the command spotting service run on ports of their own
      And listening is on with them and a dictation runs
      And the command spotting service went away during it and is back
      When the user says "stop dictation" once the service is back
      Then the dictation ends and its text is pasted

