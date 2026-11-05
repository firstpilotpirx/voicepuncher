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
      When the user says "<said>" with a pause before and after it
      Then the "<command>" command takes effect

      Examples:
        | said  | command        |
        | stop  | stop dictation |
        | enter | press enter    |

    Scenario Outline: A command said after a long time of listening is recognized
      Given the command spotting service is running
      And listening has been on for a long time with nothing said
      When the user says "<said>" with a pause before and after it
      Then the "<command>" command takes effect

      @smoke
      Examples:
        | said   | command          |
        | stop   | stop dictation   |

      @regression
      Examples:
        | said   | command          |
        | start  | start dictation  |
        | cancel | cancel dictation |
        | enter  | press enter      |
        | paste  | paste text       |

    @smoke
    Scenario: A command phrase inside a sentence is ignored while listening
      Given the command spotting service is running
      When someone says "I will enter the code when I am done" without pausing
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

    # @serial: five quiet rows at once would decode past the streaming recognizers' budget
    @serial
    Scenario Outline: A command said in a quiet voice is recognized
      Given the command spotting service is running
      When the user says "<said>" in a quiet voice with a pause before and after it
      Then the "<command>" command takes effect

      @smoke
      Examples:
        | said              | command           |
        | restart           | restart dictation |

      @regression
      Examples:
        | said              | command           |
        | start             | start dictation   |
        | cancel            | cancel dictation  |
        | stop              | stop dictation    |
        | enter             | press enter       |
        | clear             | clear all text    |

    @smoke
    Scenario: A short Russian phrase is no command
      Given the command spotting service is running
      When someone says short Russian phrases, each on its own with a pause around it
      Then no command takes effect

  Rule: Every command has a one-word short form

    Scenario Outline: A short form is recognized as its command
      Given the command spotting service is running
      When the user says "<said>" with a pause before and after it
      Then the "<command>" command takes effect

      @smoke
      Examples:
        | said           | command           |
        | cancel         | cancel dictation  |
        | paste          | paste text        |

      @regression
      Examples:
        | said           | command           |
        | start          | start dictation   |
        | restart        | restart dictation |
        | clear          | clear all text    |

    @regression
    Scenario Outline: A long form said alone takes no effect
      Given the command spotting service is running
      When someone says "<said>" in one breath while <state>
      Then no command takes effect

      Examples:
        | said           | state             |
        | stop dictation | dictating         |

    @regression
    Scenario: A long form ending in a command word counts by that word
      Given the command spotting service is running
      When someone says "press enter" in one breath while no dictation runs
      Then the "press enter" command takes effect

  Rule: "paste text" pastes the clipboard

    @smoke
    Scenario: A paste phrase inside a sentence is ignored
      Given the command spotting service is running
      When someone says "I will paste the link later" without pausing
      Then no command takes effect

  Rule: Commands are carried out in the order spoken

    @smoke
    Scenario: A message is dictated and sent by voice alone
      Given the command spotting service is running
      And listening is on
      When the user says "start", a sentence, "stop" and "enter", each command on its own
      Then the sentence is pasted into the focused app
      And the Return key is pressed after it

  Rule: Each state accepts its own commands, paste in both

    Scenario Outline: Start or let's start on its own during a dictation is ignored
      Given the command spotting service is running
      When someone says "<words>" on its own after a sentence, <level>, while dictating
      Then no command takes effect

      @smoke
      Examples:
        | words       | level              |
        | start       | at a normal level  |
        | let's start | in a quiet voice   |

      @regression
      Examples:
        | words       | level              |
        | start       | in a quiet voice   |
        | let's start | at a normal level  |

    Scenario Outline: Start ending a dictated sentence is ignored
      Given the command spotting service is running
      When someone says "<words>" in one breath, <level>, while dictating
      Then no command takes effect

      @smoke
      Examples:
        | words                         | level             |
        | we are all here, let's start  | at a normal level |
        | the meeting is about to start | in a quiet voice  |

      @regression
      Examples:
        | words                         | level             |
        | we are all here, let's start  | in a quiet voice  |
        | the meeting is about to start | at a normal level |

    Scenario Outline: Start inside a dictated sentence is ignored
      Given the command spotting service is running
      When someone says "<words>" in one breath, <level>, while dictating
      Then no command takes effect

      @smoke
      Examples:
        | words                       | level             |
        | let's start with the budget | at a normal level |
        | we start at nine tomorrow   | in a quiet voice  |

      @regression
      Examples:
        | words                       | level             |
        | let's start with the budget | in a quiet voice  |
        | we start at nine tomorrow   | at a normal level |

    @smoke
    Scenario: Stop without a dictation is ignored
      Given the command spotting service is running
      When someone says "stop" in one breath while no dictation runs
      Then no command takes effect

  Rule: A command may end an utterance

    Scenario Outline: A command right after other speech takes effect
      Given the command spotting service is running
      When someone says "<words>" in one breath while <state>
      Then the commands "<commands>" take effect in order

      @smoke
      Examples:
        | words                                    | state             | commands          |
        | We meet on Friday at ten, enter          | dictating         | press enter       |
        | Okay, start                              | no dictation runs | start dictation   |

      @regression
      Examples:
        | words                                    | state             | commands          |
        | That is all for today, stop              | dictating         | stop dictation    |
        | Forget it, cancel                        | dictating         | cancel dictation  |
        | Let me try once more, restart            | dictating         | restart dictation |
        | Okay, paste                              | no dictation runs | paste text        |

    @smoke
    Scenario: A command word inside a sentence is still no command
      Given the command spotting service is running
      When someone says "We should stop the server tonight" in one breath while dictating
      Then no command takes effect

  Rule: "start" may open an utterance

    Scenario Outline: Start followed by the dictation at once
      Given the command spotting service is running
      When someone says "<words>" in one breath while no dictation runs
      Then the commands "start dictation" take effect in order

      @smoke
      Examples:
        | words                                         |
        | Start, the meeting moved to Friday at ten     |

      @regression
      Examples:
        | words                                         |
        | Start, the build is green again               |

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
      When someone says "Restart, restart", pauses for about 0.5 s and says another sentence, and later ends the dictation
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

  # Timed like the start cues: @cues runs them with the machine otherwise idle.
  @serial @cues
  Rule: A doubled command needs no pause

    Scenario: Start start then silence starts at once
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "start start" and stays silent
      Then the start cue comes within 0.5 s of the end of "start start"

    Scenario: Stop stop in running speech stops at once
      Given the command spotting service is running
      And listening is on for a whole session
      And a dictation started by voice runs
      When someone says a sentence, "stop stop" and goes on speaking
      Then the stop sound plays within 1 s of the end of "stop stop", while the speech goes on

    Scenario: Start start inside a sentence starts at once
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "fine start start" and a sentence without a pause
      Then the start cue comes within 1 s of the end of "start start", while the sentence is still being said

  # Real-time sessions timed from the end of the command's words; @cues runs them in
  # full-check's e2e-cues stage with the machine otherwise idle, @serial keeps the other
  # scenarios' requests from queuing ahead of them.
  @serial @cues
  Rule: A voice start gives its start cue within 0.7 s of the end of its words

    Scenario: Start start followed by the dictation at once gives its cue at once
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "start start" and a sentence without a pause
      Then the start cue comes within 0.5 s of the end of "start start", while the sentence is still being said

    Scenario: Start followed by the dictation at once gives its cue at once
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "start" and a sentence without a pause
      Then the start cue comes within 0.9 s of the end of "start", while the sentence is still being said

    Scenario: Start said on its own gives its cue at once
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "start" and stays silent
      Then the start cue comes within 0.7 s of the end of "start"

    Scenario: Restart restart followed by the new dictation at once gives its cue at once
      Given the command spotting service is running
      And listening is on for a whole session
      And a dictation runs and someone has said a sentence in it
      When someone says "restart restart" and another sentence without a pause
      Then the start cue of the new dictation comes within 0.5 s of the end of "restart restart"

    Scenario: Restart said on its own gives its cue at once
      Given the command spotting service is running
      And listening is on for a whole session
      And a dictation started by voice runs
      And someone has dictated for about 20 s
      When someone says "restart" and stays silent
      Then the start cue of the new dictation comes within 0.7 s of the end of "restart"

    @voice_samples
    Scenario: The user's recorded starts give their cue at once
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      When each of the user's recorded starts is played, on its own and followed at once by a sentence
      Then at least 5 of 6 of them give the start cue within its bound of the end of the command's words

  # Proved by the e2e lock of dev-tools (dev-tools/src/locks/tests.rs: the cue stage runs with the
  # machine alone) and the load report of e2e-test/src/machine_load.rs, which a late cue's failure prints.
  Rule: The start cue is timed on an idle machine

  Rule: A doubled short form counts anywhere

    Scenario Outline: A doubled short form in the middle of speech takes effect once
      Given the command spotting service is running
      When someone says "<words>" in one breath while <state>
      Then the commands "<commands>" take effect in order

      @smoke
      Examples:
        | words                                                           | state             | commands         |
        | Wait, restart restart                                           | dictating         | restart dictation |

      @regression
      Examples:
        | words                                                           | state             | commands         |
        | We move the meeting to Friday stop stop and some more words here | dictating         | stop dictation   |
        | Fine, enter enter, thanks                                       | no dictation runs | press enter      |
        | Okay, start start                                               | no dictation runs | start dictation   |
        | Fine, paste paste                                               | no dictation runs | paste text        |

  Rule: A chain of commands is carried out in order

    Scenario Outline: A chain of commands takes effect in order
      Given the command spotting service is running
      When someone says "<words>" in one breath while <state>
      Then the commands "<commands>" take effect in order

      @smoke
      Examples:
        | words                           | state             | commands                                     |
        | Forget it, cancel, clear        | dictating         | cancel dictation, clear all text             |

      @regression
      Examples:
        | words                           | state             | commands                                     |
        | That is all, stop, clear        | dictating         | stop dictation, clear all text               |
        | That is all, stop, enter, start | dictating         | stop dictation, press enter, start dictation |
        | Forget it, cancel, enter        | dictating         | cancel dictation, press enter                |
        | Okay, paste, enter              | no dictation runs | paste text, press enter                      |
        | Okay, enter, clear              | no dictation runs | press enter, clear all text                  |

  Rule: An utterance ends after 0.3 s of silence

    @smoke @serial
    Scenario: Stop after a short breath
      Given the command spotting service is running
      And listening is on for a whole session
      And a dictation started by voice runs
      When someone says a sentence, takes a short breath and says "stop stop"
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

    # Timed like the start cues: @cues runs it with the machine otherwise idle.
    @serial @cues
    Scenario: A command in a noisy room ends at its pause
      Given the command spotting service is running
      And listening is on for a whole session
      When someone says "start" in steady room noise and stays silent
      Then the start cue comes within 0.9 s of the end of "start"

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

    # Minutes of room noise between its steps; not run, measured by hand on the streaming
    # recognizer with `just streaming-bench --recorded`.
    @wip @voice_samples
    Scenario: A hands-free session with long pauses between its steps
      Given the command spotting service is running
      And the user's own recordings of the commands exist on the machine
      And listening is on for a whole session
      When the hands-free session is spoken with the user's recorded commands and long pauses
      Then the app does everything the session said, in order

  Rule: Voice control needs the command spotting service to start

    @smoke
    Scenario: The app refuses to start without the command spotting service
      Given voice control is turned on
      And the app leaves its services to the service scripts
      And the command spotting service is not running
      When the app starts
      Then the app exits with an error that names the command spotting service and how to start it

  # Proved by app/tests/it/commandspotting_client.rs (a pause of the app's own, integration) and by
  # "A command said after a long time of listening is recognized" above (a burst after long listening).
  Rule: A command spotting service that answers late does not stop listening

  @serial @resilience
  Rule: Listening comes back when the command spotting service is back

    Scenario: Commands work again after the command spotting service restarts
      Given the command spotting service runs on ports of its own
      And listening is on with them
      And the command spotting service went away and was started again
      When the user says "enter" 5 seconds after the service answers again
      Then the Return key is pressed in the focused app
      And no listening sound played after the app started

  @serial @resilience
  Rule: The overlay shows that voice commands do not work while the service stays away

    Scenario: A long outage in standby shows the sign until the service is back
      Given the command spotting service runs on ports of its own
      And listening is on with them
      And the overlay is hidden
      When the command spotting service stays away for longer than 5 seconds
      Then the overlay shows that voice commands do not work
      And the sign goes away once the service is back

  @serial @resilience
  Rule: A dictation keeps recording while the command spotting service is away

    Scenario: "stop" ends a dictation after the service is back
      Given the command spotting service runs on ports of its own
      And listening is on with them and a dictation runs
      And the command spotting service went away during it and is back
      When the user says "stop" once the service is back
      Then the dictation ends and its text is pasted

  @serial @resilience
  Rule: Listening moves to another copy of the command spotting service

    Scenario: The command spotting copy the app listens with is killed
      Given the app started the command spotting service as two copies, and listening is on
      When the copy of the command spotting service the app listens with is killed and the user says "enter" a moment later
      Then the Return key is pressed in the focused app
      And the overlay did not show that voice commands do not work
      And the killed copy answers again within 30 seconds

  Rule: The app and the command spotting service agree on a protocol version

    @smoke
    Scenario: A client of another protocol version is refused
      Given the command spotting service is running
      When a client opens a connection to it with another protocol version
      Then the service closes the connection with a reason naming the version it speaks

    @smoke
    Scenario: A malformed message closes the connection with a reason
      Given the command spotting service is running
      When a client connected with the right protocol version sends a message that is not valid
      Then the service closes the connection with a reason saying the message could not be read

    @smoke
    Scenario: An audio frame of partial samples closes the connection with a reason
      Given the command spotting service is running
      When a client connected with the right protocol version sends an audio frame whose length is not a whole number of samples
      Then the service closes the connection with a reason saying the frame holds partial samples

  # Proved by command-spotting/tests/it/streaming/ (integration).
  Rule: Every command in audio accepted before a sync is out before the sync is answered

  Rule: The command history is on for the app's own services and off elsewhere

    @smoke
    Scenario: A service given no folder writes no entry
      Given the command spotting service runs with the streaming recognizer and is given no command history folder
      When an app streams "enter" followed by silence
      Then the streaming recognizer reports "press enter" and the command history holds no entry

  Rule: The command history stays within its size limit

    @smoke
    Scenario: The oldest command history entries go first
      Given the command spotting service runs with the streaming recognizer and keeps a command history
      And the command history's entries already fill its size limit
      When an app streams "enter" followed by silence
      Then the oldest entries are removed until the command history fits, and the newest entries, the new one included, stay

  Rule: Keeping the command history never costs a command

    @smoke
    Scenario: The command history folder cannot be written
      Given the command spotting service runs with the streaming recognizer and keeps a command history in a folder it cannot write to
      When an app streams "enter" followed by silence
      Then the streaming recognizer reports "press enter" and its log warns that the utterance was not kept

  # The streaming recognizer runs in this process on the real streaming models, as the
  # service runs it: the voice gate, the English and the Russian recognizer.
  Rule: Streaming recognizers hear commands behind a voice gate

    @smoke
    Scenario: A command said alone is reported before its pause ends
      Given the command spotting service runs with the streaming recognizer
      When an app streams "enter" followed by half a second of silence and asks for a sync
      Then the streaming recognizer reports "press enter" before it answers the sync

    @smoke
    Scenario: Silence wakes no recognizer
      Given the command spotting service runs with the streaming recognizer
      When an app streams three seconds of silence
      Then the streaming recognizer reports no command
      And the service's log shows that no streaming recognizer ran

    @smoke
    Scenario: Speech without a command is no command
      Given the command spotting service runs with the streaming recognizer
      When an app streams the sentence "the weather is nice today"
      Then the streaming recognizer reports no command

    @smoke
    Scenario: Stop opening a sentence is still no command
      Given the command spotting service runs with the streaming recognizer
      When an app streams the sentence "stop the car"
      Then the streaming recognizer reports no command

  # Real-time streams timed by the wall clock; @cues runs them with the machine otherwise idle.
  @serial @cues
  Rule: Streaming commands are reported within 0.3 s of their words

    Scenario: Start said alone is reported within its bound
      Given the command spotting service runs with the streaming recognizer
      When an app streams "start" followed by silence, in real time
      Then the streaming recognizer reports "start dictation" within 0.3 s of the end of the words by the wall clock

    Scenario: Stop said alone is reported at the pause after it
      Given the command spotting service runs with the streaming recognizer
      When an app streams "stop" followed by silence, in real time
      Then the streaming recognizer reports "stop dictation" within 0.3 s of the end of the words by the wall clock

    Scenario: Enter after a sentence is reported soon after the pause
      Given the command spotting service runs with the streaming recognizer
      When an app streams "we meet at ten enter" followed by silence, in real time
      Then the streaming recognizer reports "press enter" within 0.6 s of the end of the words by the wall clock

    # The English recognizer hears the first word as "stop"; the gate still hears the sentence.
    @smoke @voice_samples
    Scenario: A Russian sentence heard as opening with stop is no command
      Given the command spotting service runs with the streaming recognizer
      And the user's own recordings of the commands exist on the machine
      When the user's recorded "чтобы зайти в гетлаб" said while dictating is streamed to the service in real time
      Then none of the recordings is reported as a command

    # Minutes in real time: run by hand with `just streaming-bench --recorded`, never in a check.
    @wip @voice_samples
    Scenario: The user's recorded commands are reported within their bound
      Given the user's own recordings of the commands exist on the machine
      When each recorded "start" and "stop" is streamed to the service in real time
      Then at least half are reported within 0.3 s and at least 9 of 10 within 0.6 s of the end of their words

  # Timed by the wall clock while the scenario keeps every core busy itself; @cues runs it with
  # the machine otherwise idle.
  @serial @cues
  Rule: Streaming commands keep their bounds while every core is busy

    Scenario: Enter after a sentence is reported in time with every core busy
      Given the command spotting service runs with the streaming recognizer
      And every core of the machine is busy with other work
      When an app streams "we meet at ten enter" followed by silence, in real time
      Then the streaming recognizer reports "press enter" within 1.2 s of the end of the words by the wall clock

  Rule: Streaming commands are heard in the user's own voice

    # Over 7 min in real time: run by hand with `just streaming-bench --recorded`, never in a check.
    @wip @voice_samples
    Scenario: The user's recorded commands are heard
      Given the user's own recordings of the commands exist on the machine
      When every recording of the user's commands is streamed to the service
      Then at least 89 of 100 commands are reported as the command said

    @smoke @voice_samples
    Scenario: A mishearing from the history counts as its command
      Given the user's own recordings of the commands exist on the machine
      When the user's recorded "clear" that the Russian recognizer mishears is streamed again
      Then the service reports the recorded "clear"

  Rule: The fast streaming bench reproduces the service's timing

    @regression
    Scenario: A lone restart gives the same delay in both paces
      Given the command spotting service runs with the streaming recognizer
      When a lone "restart" after 20 s of dictation is streamed in real time and as fast as it decodes
      Then both paces report "restart"
      And the fast pace reports each command within 0.1 s of its real-time delay

    @regression @voice_samples
    Scenario: The user's recordings give the same delays in both paces
      Given the user's own recordings of the commands exist on the machine
      When a sample of the user's recorded commands is streamed in real time and as fast as it decodes
      Then the fast pace reports each command within 0.1 s of its real-time delay

  Rule: A command said in Russian sounds counts as the command

    @smoke
    Scenario: A Russian-accented stop is heard as stop
      Given the command spotting service runs with the streaming recognizer
      When an app streams "стоп" said by a Russian voice followed by silence
      Then the streaming recognizer reports "stop dictation" exactly once

    @smoke
    Scenario: A Russian word that is no command stays speech
      Given the command spotting service runs with the streaming recognizer
      When an app streams "привет" said by a Russian voice followed by silence
      Then the streaming recognizer reports no command

  Rule: The first recognizer to hear a command reports it once

    @smoke
    Scenario: A command both recognizers hear is reported once
      Given the command spotting service runs with the streaming recognizer
      When an app streams "enter" followed by silence
      Then the streaming recognizer reports "press enter" exactly once

    @smoke
    Scenario: A doubled command is reported once
      Given the command spotting service runs with the streaming recognizer
      When an app streams "enter enter" followed by silence
      Then the streaming recognizer reports "press enter" exactly once

  Rule: Each streaming utterance is kept with its texts and stage times

    @smoke
    Scenario: A reported command leaves an entry with its timeline
      Given the command spotting service runs with the streaming recognizer and keeps a command history
      When an app streams "enter" followed by silence
      Then the command history holds an entry with the utterance's audio
      And the entry names "press enter", the recognizer that heard it, each recognizer's texts with their times and the times of each stage

    @smoke
    Scenario: An utterance without a command is kept too
      Given the command spotting service runs with the streaming recognizer and keeps a command history
      When an app streams "the weather is nice today" followed by silence
      Then the command history holds an entry with the utterance's audio and each recognizer's texts
      And the entry names no command
