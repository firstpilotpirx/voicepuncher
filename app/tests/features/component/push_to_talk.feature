# @real_keyboard runs only with VOICEPUNCHER_REAL_KEYBOARD_IT=1: it needs Accessibility +
# Input Monitoring for the test binary, a GUI session and the keyboard to itself.
@push_to_talk @component @real_keyboard
Feature: push-to-talk

  Rule: The hotkey keeps working after a paste

    Scenario: Hotkey press after a paste is registered
      Given the user pressed and released the hotkey twice
      And the app pasted text into the focused app
      When the user presses and releases the hotkey again
      Then both the press and the release are registered
