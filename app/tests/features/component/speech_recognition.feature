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

  Rule: A microphone lost during a dictation ends it with a notice

    @serial
    Scenario: The microphone disappears mid-dictation
      Given the user is recording a hotkey dictation
      When the input device disappears
      Then the dictation ends at once
      And the speech recorded before the failure is pasted
      And the overlay shows a failure notice that the microphone failed

    @serial
    Scenario: The microphone reports an error mid-dictation
      Given the user is recording a hotkey dictation
      When the input device fails
      Then the dictation ends at once
      And the speech recorded before the failure is pasted
      And the overlay shows a failure notice that the microphone failed

    @serial
    Scenario: Nothing recorded before the failure
      Given a dictation has just started, nothing recorded yet
      When the input device fails
      Then the dictation ends at once
      And the dictation's text is not pasted
      And the overlay shows a failure notice that the dictation was lost because the microphone failed

  Rule: Service failure during a cycle drops only that cycle

    Scenario: Service stops between recordings
      Given the speech recognition service has stopped before a dictation ends
      When the user finishes that dictation and then dictates again
      Then the first dictation ends with the notice that speech recognition failed
      And the failure of the first dictation is logged
      And the text of the second dictation is pasted

  Rule: Dropped connection to the service is retried

    Scenario: Service is briefly busy
      Given the speech recognition service drops the first two connections and then answers normally
      When a recording is transcribed through the speech recognition service
      Then the transcript the service returns is passed on
      And each of the 2 dropped attempts is logged

    Scenario: Drop reported as an invalid argument
      Given the operating system reports a dropped connection as an invalid argument
      When the app judges that failure
      Then it is taken as a dropped connection, whose recording is sent again

    Scenario: Service keeps dropping the connection
      Given the speech recognition service drops every connection
      When a recording is transcribed through the speech recognition service
      Then the recording is sent 4 times in total and the transcription fails

    Scenario: Service answers with an error
      Given the speech recognition service answers every request with an error
      When a recording is transcribed through the speech recognition service
      Then the recording is sent 3 times in total and the transcription fails

    Scenario: A chunk whose copy is killed is transcribed by another copy
      Given the speech recognition service runs as two copies, and the first copy to get a request drops it
      When a recording is transcribed through the speech recognition service
      Then the transcript the service returns is passed on
      And another copy answered, and the copy that failed the first request got no second attempt

  Rule: A request that times out or gets an error answer is retried

    Scenario: The first request of a chunk hangs
      Given the speech recognition service never answers the first request and answers the next one normally
      When a recording is transcribed through the speech recognition service
      Then the transcript the service returns is passed on
      And the hung attempt is logged

    Scenario: One error answer is retried
      Given the speech recognition service answers the first request with an error and the next one normally
      When a recording is transcribed through the speech recognition service
      Then the transcript the service returns is passed on
      And the error answer is logged

    Scenario: The attempt after a hung one goes to another copy
      Given the speech recognition service runs as two copies, and the first copy to get a request never answers it
      When a recording is transcribed through the speech recognition service
      Then the transcript the service returns is passed on
      And another copy answered, and the copy that failed the first request got no second attempt

  Rule: The wait for a transcript follows the length of the audio

    Scenario: A hung short chunk is given up quickly
      Given the speech recognition service never answers any request
      When a 5-second recording is transcribed through the speech recognition service
      Then the transcription fails after 3 attempts, given up within 5 seconds
      And every attempt was logged

  Rule: No speech yields no text

    Scenario: Service hears no speech
      Given the speech recognition service returns no text for a recording
      When a recording is transcribed through the speech recognition service
      Then no text is produced and no failure is reported

  Rule: Recording after a long pause reaches the service at once

    Scenario: Service closed the idle connection during a long pause
      Given a recording was transcribed through the speech recognition service
      And the speech recognition service has since closed the connection that recording used
      When a recording is transcribed through the speech recognition service
      Then the transcript the service returns is passed on
      And no dropped attempt is logged, and each recording was sent once

  Rule: The same dictation audio gives the same text

    Scenario: Hard audio transcribed twice gives the same text
      Given the speech recognition service answers every request
      And a recording of speech under high noise
      When the recording is transcribed twice in a row
      Then both requests are the same and ask for no sampling randomness
      And both transcripts are the same text

  Rule: A check transcribes one recording file at a time

    Scenario: A copy that does not answer in time is reported with the load
      Given the speech recognition service runs as two copies, and the first copy to get a request never answers it
      When a recording is transcribed through the speech recognition service
      And the check reports what the copies of the service did
      Then the report names the copy that did not answer, the busiest processes on the machine and every check that held a lock

  Rule: A recording loses no audio under load

    Scenario: Audio delivered while the recording is held up is kept
      Given a recording is set up on a microphone that records at 48 kHz
      When the part that collects the audio is held up for 5 seconds while the microphone keeps delivering
      Then the audio delivered meanwhile is in the recording in order, with nothing missing

    Scenario: Audio dropped is reported once
      Given a recording is set up on a microphone whose audio the recording can hold back only briefly
      When the microphone delivers a second of audio at once and then goes on
      Then the recording goes on and the log reports once how much audio was dropped

  Rule: The audio sent to speech recognition is exactly what was recorded

    Scenario: The final chunk ends where the dictation ends
      Given a recording is set up on a microphone that records at 48 kHz
      When the microphone delivers a second of steady sound and the user stops the dictation
      Then the recording is exactly as long as the sound and loud to its last sample

    Scenario: Chunks of one dictation join without a gap
      Given a recording is set up on a microphone that records at 48 kHz
      When the user dictates three and a half seconds cut into chunks of a second and stops
      Then the chunks laid end to end are the whole recording, with no gap or repeated samples
