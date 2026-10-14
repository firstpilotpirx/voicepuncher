@local_services @e2e
Feature: local-services

  @smoke @serial
  Rule: The service scripts start the services named

    Scenario: Only the speech recognition service is started
      Given no local service runs on the chosen ports
      When the user starts only the speech recognition service with the service scripts
      Then the speech recognition service answers and nothing answers on the text refinement and command spotting ports

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
  Rule: The speech recognition service runs as several copies behind one address

    Scenario: Requests that arrive at once are shared by the copies
      Given the service scripts started the speech recognition service without a number of copies
      When several transcription requests arrive at once
      Then every request gets its transcript, and each of the three copies transcribed some of them

  @smoke @serial
  Rule: A copy that stops does not stop the service

    Scenario: One copy of the speech recognition service dies
      Given the service scripts started the speech recognition service as several copies
      When one copy exits on its own
      Then the service keeps answering transcription requests, and that copy answers again within 30 seconds

  @smoke @serial
  Rule: Short speech is transcribed ahead of long speech

    Scenario: A short utterance overtakes long speech that waits
      Given the speech recognition service runs as two copies, both busy with long speech, and more long speech waits
      When a short utterance arrives
      Then the short utterance is transcribed before the long speech that waited

  @smoke @serial
  Rule: Stopping the speech recognition service stops every copy

    Scenario: The user stops the speech recognition service that runs as several copies
      Given the service scripts started the speech recognition service as several copies
      When the user stops it with the service scripts
      Then nothing answers on its port, and no copy of it is left

  @smoke @serial
  Rule: Several copies that cannot start are reported and leave nothing behind

    Scenario: The second of several copies cannot start
      Given the speech recognition service is to run as several copies, and its second copy cannot start
      When the service scripts start the speech recognition service
      Then the start fails naming the service and the log of the copy that failed, and neither the copy that started nor anything in front of it is left
