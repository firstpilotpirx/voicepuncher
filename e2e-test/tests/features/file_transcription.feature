@file_transcription @e2e
Feature: file-transcription

  Rule: A recording file runs through the whole pipeline

    # Without refinement, silence of 1, 10 and 60 s runs in speech_recognition.feature,
    # "Silence produces a lone "you"", whose steps also fail on a startup failure.
    @smoke
    Scenario Outline: Silence is processed without a startup failure
      Given a recording of silence lasting <seconds> seconds
      When the user transcribes the recording file <refinement> refinement
      Then the app does not exit with a startup failure

      Examples:
        | seconds | refinement |
        | 1       | with       |
        | 10      | with       |
        | 60      | with       |

  Rule: Transcription meets the accuracy bar

    @accuracy
    Scenario: Benchmark corpus is transcribed within the accuracy bar
      Given a fixed sample of 100 recordings of the benchmark corpus, half Russian and half English, with reference transcripts
      When the user transcribes every recording of the sample with refinement
      Then the mean word error rate is below 30 %
