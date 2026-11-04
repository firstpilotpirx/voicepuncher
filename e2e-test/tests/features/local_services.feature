@local_services @e2e
Feature: local-services

  @smoke @serial
  Rule: The service scripts start the services named

    Scenario: Only the speech recognition service is started
      Given no local service runs on the chosen ports
      When the user starts only the speech recognition service for dictation with the service scripts
      Then the speech recognition service for dictation answers and nothing answers on the port of the command spotting service

  @smoke @serial
  Rule: A copy of a speech recognition service is ready once it has transcribed speech

    Scenario: The service scripts report a copy ready after its first transcription
      Given no local service runs on the chosen ports
      When the service scripts start the speech recognition service for dictation
      Then each copy is reported ready with the time of its first transcription, and the copy's log shows that transcription before the report

  @smoke @serial
  Rule: A service that stops is started again

    Scenario: The speech recognition service dies while voice control runs
      Given the service scripts started the speech recognition service
      When its process exits on its own
      Then the service answers on its port again within 30 seconds, and its log keeps the output of the run that ended followed by a note of the restart

  @serial
  Rule: A stopped service stays stopped

    @smoke
    Scenario: The user stops the speech recognition service
      Given the service scripts started the speech recognition service
      When the user stops it with the service scripts
      Then nothing answers on its port, still 5 seconds later, and no process of it is left

    @regression
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

  @serial
  Rule: The speech recognition service runs as several copies found from its address

    @smoke
    Scenario: Requests that arrive at once are shared by the copies
      Given the service scripts started the speech recognition service for dictation without a number of copies
      When several transcription requests arrive at once
      Then every request gets its transcript, and each of the three copies transcribed some of them

    @regression
    Scenario: Requests that arrive at once are shared by the copies while the machine is busy
      Given the service scripts started the speech recognition service for dictation without a number of copies, and another run keeps the machine's processor busy
      When several transcription requests arrive at once
      Then every request gets its transcript within the time the queued transcriptions take on the busy machine, and each of the three copies transcribed some of them

    @regression
    Scenario: A request without a transcript names the machine's load
      Given the service scripts started the speech recognition service for dictation without a number of copies
      When several transcription requests arrive at once and one of them gets no transcript in time
      Then the failure names that request, the copies it waited on and the busiest processes on the machine

    @regression
    Scenario: Requests wait their turn when every copy is busy
      Given the speech recognition service for dictation runs as two copies, both busy with long speech
      When two more requests arrive one after another
      Then each starts on the first copy that frees, in the order they arrived, and no copy transcribes two requests at once

  @serial
  Rule: Copies of a speech recognition service are found from its address

    @smoke
    Scenario: Three copies are found from the service address
      Given the service scripts started the speech recognition service for dictation as three copies
      When a client connects to the service by its address
      Then the client sends its requests to the three copies, and nothing listens on the service's own port

    @regression
    Scenario: A single copy on the service's own port is found
      Given the service scripts started the speech recognition service for dictation as one copy
      When a client connects to the service by its address
      Then the client sends its requests to that copy on the service's own port

    @regression
    Scenario: Another program on the service's own port is passed over
      Given the service scripts started the speech recognition service for dictation as three copies
      And another program that is no speech recognition service listens on the service's own port
      When a client connects to the service by its address
      Then the client sends its requests to the three copies, and the other program receives no transcription request

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
      When an app streams speech to it through several of the scripts' checks while every core of the machine is busy
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

  @serial
  Rule: The app starts the services it needs

    @smoke
    Scenario: A real launch leaves no service behind at quit
      Given no local service runs on the chosen ports and the models are on disk
      When the app launches, its services become ready and the app is told to end
      Then each started service answered on its port while the app ran
      And no process of them is left after the app ended

    @regression
    Scenario: A quit from the system stops the owned services
      Given no local service runs on the chosen ports and the models are on disk
      When the app launches, its services become ready and the system tells the app to quit
      Then each started service answered on its port while the app ran
      And the app's log names the quit from the system
      And no process of them is left after the app ended

  @serial @resilience
  Rule: An adopted service that stops is replaced by the app's own

    Scenario: An adopted command spotting copy that stops is replaced
      Given voice control is on and the app adopted both copies of the command spotting service an app run left running
      When the process of one adopted command spotting copy ends
      Then within 30 seconds the app has started its own command spotting copy on that port and it answers
      And the log names the command spotting copy, its port and that it stopped
      And a voice command said to that copy is heard

  @serial @resilience
  Rule: An owned service that stops is started again

    Scenario: An owned speech recognition copy that stops answering is restarted
      Given the app started the speech recognition service for dictation as two copies and they are ready
      When one copy stops answering without exiting
      Then within 30 seconds the app has killed that copy and started it again, and it answers on its port
      And the other copies were never restarted
