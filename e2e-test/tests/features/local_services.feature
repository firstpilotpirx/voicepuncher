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

  @smoke @serial
  Rule: A service that cannot start is reported, not retried forever

    Scenario: The speech recognition model is not a model
      Given the speech recognition model file is not a model
      When the service scripts start the speech recognition service
      Then the start fails within 15 seconds naming the service and its log, and no process of it is left
