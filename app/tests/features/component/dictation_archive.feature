# The pipeline runs on its own thread with stand-ins for the recorder, speech recognition,
# the paster and the overlay, and keeps its dictations in the real archive
# in a temporary folder; the scenarios read the entries back from that folder. The steps
# block while they watch the pipeline, so @serial keeps other scenarios from stalling them.
@dictation_archive @component @serial
Feature: dictation-archive

  Rule: Every dictation is kept in an archive entry of its own

    Scenario: A pasted dictation is archived with its audio and texts
      Given the archive is on
      When the user dictates a phrase with the hotkey and it is pasted
      Then a new archive entry holds the audio of the dictation, the raw transcript speech recognition returned, and the text that was pasted

    Scenario: A long dictation keeps the text of every chunk
      Given the archive is on
      When the user dictates long enough for the dictation to be transcribed in several chunks
      Then its archive entry holds the text of each chunk in the order spoken, the stitched raw transcript and the audio of the whole dictation

    Scenario: Two dictations get two entries
      Given the archive is on
      When the user dictates two phrases one after the other
      Then the archive holds two entries, each with only its own audio and texts, the later one sorting after the earlier one

  Rule: Dictations that paste nothing are archived with the reason

    Scenario: Speech recognition fails
      Given the archive is on and the speech recognition service fails
      When the user dictates a phrase
      Then its entry holds the audio and says speech recognition failed, with no pasted text

    Scenario: Only command words were transcribed
      Given the archive is on and voice control is on
      When the user starts a dictation by voice and stops it without saying anything else
      Then its entry holds the audio and says only command words were transcribed

    Scenario: A cancelled dictation leaves no entry
      Given the archive is on
      When the user cancels a dictation
      Then the archive holds no entry for it

    Scenario: A dictation dropped by restarting leaves no entry
      Given the archive is on and voice control is on
      When the user starts a dictation by voice, restarts it, dictates a phrase and stops it by voice
      Then the archive holds one entry, with the audio of the restarted dictation only

    Scenario: A press shorter than the minimum press time leaves no entry
      Given the archive is on
      When the user records for less than the minimum press time
      Then the archive holds no entry for it

  Rule: An entry carries what is needed to compare runs

    Scenario: A voice-started dictation is marked as such
      Given the archive is on and voice control is on
      When the user starts a dictation by voice, dictates a phrase and stops it by voice
      Then its entry says it started by voice and names the app version and the dictation model

    Scenario: An entry has no refinement fields
      Given the archive is on
      When the user dictates a phrase
      Then its entry names no text refinement model and no refinement outcome

  Rule: The archive stays within its size limit

    Scenario: The oldest entries go first
      Given an archive whose entries already fill its size limit
      When a new entry is written
      Then the oldest entries are removed until the archive fits, and the newest entries, the new one included, stay

  Rule: Archiving never costs a dictation

    Scenario: The archive folder cannot be written
      Given an archive folder the app cannot write to
      When the user dictates a phrase
      Then the phrase is pasted as usual and the log warns that the dictation was not archived
