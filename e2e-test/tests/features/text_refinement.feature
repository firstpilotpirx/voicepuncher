@text_refinement @e2e
Feature: text-refinement

  Rule: Refinement through a local text refinement service

    @smoke
    Scenario: Transcript is refined by the service
      Given the text refinement service answers with corrected text
      When the user transcribes a recording of speech with refinement
      Then the text the service returned is pasted

    @smoke
    Scenario: App starts without a local model file
      Given the text refinement service is running
      And no language model is on disk for the app
      When the user starts the app with refinement
      Then the app starts and is ready to record

  Rule: Transcript is passed as data, not as instructions

    @smoke
    Scenario: Dictated text sounds like a request
      Given the text refinement service answers with corrected text
      When the user dictates a sentence that asks for a translation
      Then the text refinement service receives the sentence as the text to correct, apart from the correction instructions

  Rule: Text refinement service stays on this machine

    @smoke
    Scenario: Remote service address is rejected
      Given the text refinement service address is on another host
      When the user starts the app with refinement
      Then the app exits before recording anything and explains that only a local text refinement service is allowed

  Rule: Unavailable text refinement service is reported at startup

    @smoke
    Scenario: Service is not running at startup
      Given no text refinement service is listening at the configured address
      When the user starts the app with refinement
      Then the app exits within 3 seconds with a message naming the text refinement service address and how to start it

    @smoke
    Scenario: Configured model is not offered
      Given the text refinement service offers only models other than the configured one
      When the user starts the app with refinement
      Then the app exits with a message listing the models the service offers

  Rule: Refinement failure falls back to the raw transcript

    @smoke
    Scenario: Service stops between recordings
      Given the text refinement service stops after the app has started
      When the user transcribes a recording of speech with refinement
      Then the raw transcript is pasted and the failure is logged

    @smoke
    Scenario: Service hits its output limit
      Given the text refinement service stops generating at its output limit
      When the user transcribes a recording of speech with refinement
      Then the raw transcript is pasted instead of the cut-off text

  Rule: Refinement can be disabled

    @smoke
    Scenario: Running without refinement
      Given no text refinement service is listening at the configured address
      When the user transcribes a recording of speech without refinement
      Then the raw transcript is pasted

  Rule: Model reasoning is never pasted

    @smoke
    Scenario: Reasoning model is configured
      Given the text refinement service answers with a reasoning block followed by corrected text
      When the user transcribes a recording of speech with refinement
      Then only the corrected text is pasted

  Rule: Outdated text refinement settings are reported

    @smoke
    Scenario: Config still points at a model file
      Given the configuration sets a language model file path
      When the user starts the app with refinement
      Then the app exits with a message saying the language model is now chosen when starting the text refinement service and naming its address setting
