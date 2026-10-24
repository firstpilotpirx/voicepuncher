@push_to_talk @component
Feature: push-to-talk

  Rule: The hotkey keeps working after a paste

    # @real_keyboard runs only with VOICEPUNCHER_REAL_KEYBOARD_IT=1: it needs Accessibility +
    # Input Monitoring for the test binary, a GUI session and the keyboard to itself.
    @real_keyboard
    Scenario: Hotkey press after a paste is registered
      Given the user pressed and released the hotkey twice
      And the app pasted text into the focused app
      When the user presses and releases the hotkey again
      Then both the press and the release are registered

  Rule: Holding the hotkey records and releasing it pastes

    Scenario: A held hotkey records and a release pastes
      Given the app is ready in Hold mode and a text field has keyboard focus
      When the user holds the hotkey, speaks a phrase and releases the hotkey
      Then a recording ran while the hotkey was held
      And the phrase is pasted into the text field after the release

    Scenario: A short press records nothing
      Given the app is ready in Hold mode with a shortest recording no press reaches
      When the user presses and releases the hotkey at once
      Then nothing is transcribed or pasted

  Rule: Nothing is asked in a terminal

    Scenario: A first launch asks nothing
      Given there is no configuration file yet
      When the app starts with nothing on its standard input
      Then it creates the configuration file and goes on without reading its standard input

    Scenario: The setup option is gone
      When the user starts the app with `--setup`
      Then the app refuses the unknown option with exit code 2

  Rule: The hotkey listener survives every key press

    Scenario: The listener reads key codes only
      Given the app is built
      When its hotkey listener is inspected
      Then it uses no keyboard library that turns key codes into typed text, and no call to the input-source service

    Scenario: A modifier hotkey is pressed and released
      Given the hotkey is right Option
      When right Option goes down and up while left Option stays up
      Then the listener reports a press and then a release

    Scenario: The other side's modifier is not the hotkey
      Given the hotkey is right Option
      When left Option goes down and up
      Then the listener reports nothing
