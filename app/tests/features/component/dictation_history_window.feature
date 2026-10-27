@dictation_history_window @component
Feature: dictation-history-window

  Rule: The history window lists the latest dictations newest first

    Scenario: Dictations are listed newest first with their time
      Given the archive holds three dictations with text, made one after the other
      When the user opens the history window
      Then it lists the three texts, the latest first, each with the local time its dictation started

    Scenario: A dictation that pasted nothing shows its transcript
      Given the archive holds a dictation whose paste failed and one with only its raw transcript
      When the user opens the history window
      Then the first row shows the text that was to be pasted and the second the raw transcript

    Scenario: A dictation without text has no row
      Given the archive holds a dictation on which speech recognition failed and one with text
      When the user opens the history window
      Then it lists only the dictation with text

    Scenario: Only the newest 300 dictations are listed
      Given the archive holds 305 dictations with text
      When the user opens the history window
      Then it lists the newest 300 and none of the 5 oldest

  Rule: A row's text is copied to the clipboard

    Scenario: Copy puts exactly the row's text on the clipboard
      Given the history window lists two dictations
      When the user copies the second row
      Then the clipboard holds exactly the text of the second row

  Rule: The open window shows a new dictation at once

    Scenario: A new dictation appears while the window is open
      Given the history window is open and lists one dictation
      When a new dictation is archived and the window checks the archive
      Then the window lists the new dictation as its first row, above the earlier one

  Rule: The history window explains an empty list

    Scenario: The archive is off
      Given the archive is turned off in the settings
      When the user opens the history window
      Then the window says the archive is off and history is not kept

    Scenario: Nothing archived yet
      Given the archive is on and holds no dictation
      When the user opens the history window
      Then the window says no dictation has been kept yet

  Rule: An unreadable entry does not stop the history window

    Scenario: A damaged entry is skipped
      Given the archive holds two dictations with text and one damaged entry between them
      When the user opens the history window
      Then it lists the two dictations and the log warns about the damaged entry
