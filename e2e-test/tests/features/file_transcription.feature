@file_transcription @e2e
Feature: file-transcription

  Rule: A recording file runs through the whole pipeline

    # Silence of 1, 10 and 60 s also runs in speech_recognition.feature, "Silence produces a
    # lone "you"", whose steps also fail on a startup failure.
    @smoke @serial
    Scenario Outline: Silence is processed without a startup failure
      Given a recording of silence lasting <seconds> seconds
      When the user transcribes the recording file
      Then the app does not exit with a startup failure
      And no permission is checked

      Examples:
        | seconds |
        | 1       |
        | 10      |
        | 60      |

  Rule: Transcription meets the accuracy bar

    @accuracy
    Scenario: Benchmark corpus is transcribed within the accuracy bar
      Given a fixed sample of 100 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
      When the user transcribes every recording of the sample
      Then the mean word error rate is no worse than the accepted baseline
