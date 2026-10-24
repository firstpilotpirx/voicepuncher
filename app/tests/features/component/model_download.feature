@model_download @component
Feature: model-download

  Rule: Missing models are listed before anything downloads

    Scenario: A launch with the dictation model missing
      Given the app manages its services and the dictation model is not in the models folder
      When the app launches
      Then the models check lists the dictation model as Missing with its size and purpose
      And the files that are present are listed as Present
      And nothing is downloaded and no service is started

    Scenario: A launch with every model present shows no window
      Given every model file the configuration needs is in the models folder
      When the app launches
      Then the runtime never waits for models and the services start

  Rule: Download fetches, verifies and places each file

    Scenario: Every missing file is downloaded and placed
      Given two needed files are missing
      When the user chooses Download
      Then each file is fetched in turn with its progress shown, its checksum verified, and moved to its name in the models folder
      And no part file is left

  Rule: A download resumes where it stopped

    Scenario: Cancel and download again
      Given a file was half downloaded when the user chose Cancel
      When the user chooses Download again
      Then only the remaining bytes are fetched
      And the placed file's checksum matches the catalog

  Rule: A file whose checksum does not match is downloaded again

    Scenario: A corrupted part is downloaded again
      Given a part whose bytes were changed
      When the file is downloaded
      Then the mismatch is found, the file is fetched again from the start and placed with a matching checksum

    Scenario: A file that never matches is reported
      Given the server sends bytes that never match the catalog checksum
      When the file is downloaded
      Then the file is fetched twice and then reported Failed with checksum mismatch
      And nothing is placed under its name

  Rule: Without internet the user is told and nothing is deleted

    Scenario: No connection
      Given no connection can be made to the model hosts
      When the user chooses Download
      Then each missing file is tried 4 times with pauses of 2, 4 and 8 seconds and then reported Failed with no internet connection
      And the files that were present are still present

  Rule: Not enough space is refused before starting

    Scenario: A disk that is too full
      Given 3 GB are missing and 1 GB is free
      When the user chooses Download
      Then nothing is fetched
      And the download reports that 3 GB are needed and 1 GB is free

  Rule: The services start when the last file is present

    Scenario: The last file arrives
      Given the runtime waits for models
      When the last missing file is downloaded and placed
      Then the runtime starts the services and becomes Ready once they answer

  Rule: A model is present by its place and size

    Scenario: A file of the wrong size is missing
      Given the dictation model file in the models folder is shorter than the catalog size
      When the app checks the models
      Then the dictation model is Missing

    Scenario: A model named by its variable counts as present
      Given the variable for the dictation model names an existing file outside the models folder
      When the app checks the models
      Then the dictation model is Present

  Rule: The same download runs headless

    Scenario: A headless run with every file present
      Given a folder that holds every catalog file
      When the user runs the headless download of the models into that folder
      Then nothing is fetched and every file is reported present
      And the command exits 0

    Scenario: A headless download that fails
      Given a folder that misses one catalog file and a host for it that refuses connections
      When the user runs the headless download of the models into that folder
      Then the command names the failed file
      And the command exits 1
