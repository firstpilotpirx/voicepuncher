@speech_recognition @e2e
Feature: speech-recognition

  Rule: Transcription through a local speech recognition service

    @smoke
    Scenario: App starts without a local model file
      Given the speech recognition service is running
      And no speech recognition model is on disk for the app
      When the user starts the app
      Then the app does not exit with a startup failure

  Rule: Speech recognition service stays on this machine

    @smoke
    Scenario: Remote service address is rejected
      Given the speech recognition service address is on another host
      When the user starts the app
      Then the app exits before recording anything and explains that only a local service is allowed

  Rule: Unavailable speech recognition service is reported at startup

    @smoke
    Scenario: Service is not running at startup
      Given no speech recognition service is listening at the configured address
      When the user starts the app
      Then the app exits within 3 seconds with a message naming the address and how to start the service

  Rule: Outdated speech recognition settings are reported

    @smoke
    Scenario: Config still points at a model file
      Given the configuration sets a speech recognition model file path
      When the user starts the app
      Then the app exits with a message saying the model is now chosen when starting the service and naming the service address setting
