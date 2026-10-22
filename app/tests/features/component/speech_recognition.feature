# The pipeline runs with stand-ins for the microphone and the speech recognition service:
# the recorder delivers live chunks, the service answers each chunk with a fixed transcript.
@speech_recognition @component
Feature: speech-recognition

  Rule: Long recordings are transcribed in chunks

    Scenario: Dictation longer than the chunk threshold
      Given the user dictates for longer than the chunk threshold
      And the service transcribes the overlap of every two chunks the same way
      When the user stops the dictation
      Then the pasted text contains every spoken word once, in order

    Scenario Outline: Chunks transcribe the overlap differently
      Given a <language> dictation longer than the chunk threshold
      And the service transcribes the overlap differently in the two chunks: <difference>
      When the user stops the dictation
      Then the overlapping words appear once in the pasted text
      And the words before and after the overlap are kept

      Examples:
        | language | difference                         |
        | Russian  | other punctuation and case         |
        | Russian  | another spelling                   |
        | Russian  | a word cut short at the chunk edge |
        | Russian  | a different reading of a word      |
        | English  | other punctuation and case         |
        | English  | another spelling                   |
        | English  | a word cut short at the chunk edge |
        | English  | a different reading of a word      |

    Scenario: Chunks share no words
      Given a dictation longer than the chunk threshold whose chunk transcripts share no words at the seam
      When the user stops the dictation
      Then the pasted text contains every word of every chunk, in order

  Rule: A lost chunk does not lose the rest of the dictation

    Scenario: The middle chunk of a long dictation fails
      Given a dictation long enough for three chunks
      And the speech recognition service fails every attempt for the second chunk
      When the user stops the dictation
      Then the text of the first and third chunks is pasted
      And the lost second chunk is logged

  Rule: A dead transcription worker loses only its dictation

    Scenario: The worker dies mid-dictation
      Given the live transcription worker of a dictation stops without a result
      When the user finishes that dictation and then dictates again
      Then the first dictation ends with the notice that speech recognition failed
      And the text of the second dictation is pasted

  Rule: Stopping a dictation waits a bounded time for its transcript

    Scenario: The speech recognition request never answers
      Given the speech recognition service never answers the last chunk of a dictation
      When the user finishes the dictation and presses the hotkey during the wait
      Then the dictation ends with the notice that speech recognition failed once the wait is over
      And the hotkey press made during the wait starts a new recording
