@dictation_archive @e2e
Feature: dictation-archive

  Rule: The archive is on for dictations and opt-in for recording files

    @smoke @serial
    Scenario: Archive turned off
      Given a recording of the user saying a phrase
      And an archive folder named on the command line
      And the user turned the archive off
      When the user transcribes the recording file in a home of its own
      Then no archive entry is written

    @smoke @serial
    Scenario: A recording file is archived into a named folder
      Given a recording of the user saying a phrase
      And an archive folder named on the command line
      When the user transcribes the recording file in a home of its own
      Then that folder holds one entry with the file's audio, the raw transcript and the printed text
      And the entry is marked as started from a recording file

    @smoke @serial
    Scenario: A recording file without a named folder is not archived
      Given a recording of the user saying a phrase
      And no archive folder named on the command line
      When the user transcribes the recording file in a home of its own
      Then no archive entry is written
