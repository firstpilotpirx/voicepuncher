@local_services @e2e
Feature: local-services

  @smoke @serial
  Rule: The service scripts start the services named

    Scenario: Only the speech recognition service is started
      Given no local service runs on the chosen ports
      When the user starts only the speech recognition service for dictation with the service scripts
      Then the speech recognition service for dictation answers and nothing answers on the ports of the speech recognition service for commands, the text refinement service and the command spotting service

  @smoke @serial
  Rule: A copy of a speech recognition service is ready once it has transcribed speech

    Scenario: The service scripts report a copy ready after its first transcription
      Given no local service runs on the chosen ports
      When the service scripts start the speech recognition service for commands
      Then each copy is reported ready with the time of its first transcription, and the copy's log shows that transcription before the report

  @smoke @serial
  Rule: Voice commands are confirmed by a speech recognition service of their own

    Scenario: A command is confirmed by the speech recognition service for commands
      Given the models that the service scripts download are on disk
      And the user started all services with the service scripts
      When an app streams "press enter" to the command spotting service
      Then the command spotting service reports "press enter"
      And the speech recognition service for commands answered its confirmation with the small model, and the speech recognition service for dictation got no request

  @smoke @serial
  Rule: A service that stops is started again

    Scenario: The speech recognition service dies while voice control runs
      Given the service scripts started the speech recognition service
      When its process exits on its own
      Then the service answers on its port again within 30 seconds, and its log keeps the output of the run that ended followed by a note of the restart

  @smoke @serial
  Rule: A stopped service stays stopped

    Scenario: The user stops the speech recognition service
      Given the service scripts started the speech recognition service
      When the user stops it with the service scripts
      Then nothing answers on its port, still 5 seconds later, and no process of it is left

    Scenario: The user stops a single speech recognition service after copies were left listed
      Given a list of copies is left from an earlier start, and the service scripts started the speech recognition service as one copy
      When the user stops it with the service scripts
      Then nothing answers on its port, still 5 seconds later, and no process of it is left

  @smoke @serial
  Rule: A service that cannot start is reported, not retried forever

    Scenario: The speech recognition model is not a model
      Given the speech recognition model file is not a model
      When the service scripts start the speech recognition service
      Then the start fails within 15 seconds naming the service and its log, and no process of it is left

  @smoke @serial
  Rule: The speech recognition service runs as several copies found from its address

    Scenario: Requests that arrive at once are shared by the copies
      Given the service scripts started the speech recognition service for dictation without a number of copies
      When several transcription requests arrive at once
      Then every request gets its transcript, and each of the three copies transcribed some of them

    Scenario: Requests that arrive at once are shared by the copies while the machine is busy
      Given the service scripts started the speech recognition service for dictation without a number of copies, and another run keeps the machine's processor busy
      When several transcription requests arrive at once
      Then every request gets its transcript within the time the queued transcriptions take on the busy machine, and each of the three copies transcribed some of them

    Scenario: A request without a transcript names the machine's load
      Given the service scripts started the speech recognition service for dictation without a number of copies
      When several transcription requests arrive at once and one of them gets no transcript in time
      Then the failure names that request, the copies it waited on and the busiest processes on the machine

    Scenario: Requests wait their turn when every copy is busy
      Given the speech recognition service for dictation runs as two copies, both busy with long speech
      When two more requests arrive one after another
      Then each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once

  @smoke @serial
  Rule: Copies of a speech recognition service are found from its address

    Scenario: Three copies are found from the service address
      Given the service scripts started the speech recognition service for dictation as three copies
      When a client connects to the service by its address
      Then the client sends its requests to the three copies, and nothing listens on the service's own port

    Scenario: A single copy on the service's own port is found
      Given the service scripts started the speech recognition service for dictation as one copy
      When a client connects to the service by its address
      Then the client sends its requests to that copy on the service's own port

  @smoke @serial
  Rule: A copy that stops does not stop the service

    Scenario: One copy of the speech recognition service dies
      Given the service scripts started the speech recognition service as several copies
      When one copy exits on its own
      Then the service keeps answering transcription requests, and that copy answers again within 30 seconds

  @smoke @serial
  Rule: Stopping the speech recognition service stops every copy

    Scenario: The user stops the speech recognition service that runs as several copies
      Given the service scripts started the speech recognition service as several copies
      When the user stops it with the service scripts
      Then no copy of it is left, and nothing answers on any of its ports

  @smoke @serial
  Rule: Several copies that cannot start are reported and leave nothing behind

    Scenario: The second of several copies cannot start
      Given the speech recognition service is to run as several copies, and its second copy cannot start
      When the service scripts start the speech recognition service
      Then the start fails naming the service and the log of the copy that failed, and the copy that started is not left

  @serial @resilience
  Rule: A service that stops answering is started again

    Scenario: The command spotting service hangs
      Given the service scripts started the command spotting service
      When its process stops answering without exiting
      Then the service answers on its port again within 30 seconds, and its log notes that it was restarted because it stopped answering

    Scenario: A busy service is not restarted
      Given the service scripts started the command spotting service
      When an app streams speech to it through several of the scripts' checks while the speech recognition service for commands answers its confirmations slowly
      Then the service is not restarted

  @serial @resilience
  Rule: A request given up by its client does not hold a copy

    Scenario: The request after an abandoned one goes to another copy
      Given the speech recognition service for dictation runs as two copies
      And a client gave up on a request that one copy is still transcribing
      When the client sends its next request
      Then the other copy answers it, and the first copy gets a new request only after it answered the abandoned one

    Scenario: An abandoned request is not transcribed
      Given the speech recognition service for dictation runs as two copies, both busy with long speech
      And a third request waits and its client gives up
      When the copies become free
      Then the abandoned request is not sent to any copy


  @serial @resilience
  Rule: Speech that is no command is confirmed quickly

    # frozen: flakes beside another full-check (2026-10-20..23, e2e-resilience, six branches); fixed separately
    @wip
    Scenario: Russian speech that makes speech recognition repeat itself
      Given voice control runs and the speech recognition service for commands is otherwise idle
      When the user says Russian counting and filler words
      Then every confirmation of that speech is answered in under 2 seconds

    Scenario: A command right after such speech is carried out
      Given voice control runs and the speech recognition service for commands is otherwise idle
      When the user says Russian counting and filler words and then "press enter"
      Then the command spotting service reports "press enter"

  @smoke @serial
  Rule: The app starts the services it needs

    Scenario: A real launch leaves no service behind at quit
      Given no local service runs on the chosen ports and the models are on disk
      When the app launches, its services become ready and the app is told to end
      Then each started service answered on its port while the app ran
      And no process of them is left after the app ended

    Scenario: A quit from the system stops the owned services
      Given no local service runs on the chosen ports and the models are on disk
      When the app launches, its services become ready and the system tells the app to quit
      Then each started service answered on its port while the app ran
      And the app's log names the quit from the system
      And no process of them is left after the app ended

  @serial @resilience
  Rule: An owned service that stops is started again

    Scenario: An owned speech recognition copy that stops answering is restarted
      Given the app started the speech recognition service for dictation as two copies and they are ready
      When one copy stops answering without exiting
      Then within 30 seconds the app has killed that copy and started it again, and it answers on its port
      And the other copies were never restarted
