## MODIFIED Requirements

### Requirement: Transcription through a local speech recognition service
The system SHALL obtain every raw transcript from the configured speech recognition service using the OpenAI Audio Transcriptions API, and SHALL NOT load a speech recognition model into its own process.

#### Scenario: Recording is transcribed by the service
- **WHEN** the user finishes a recording while the speech recognition service is running
- **THEN** the transcript the service returns is passed on for pasting

#### Scenario: App starts without a local model file
- **WHEN** the user starts the app with no speech recognition model on disk and the service running
- **THEN** the app starts and is ready to record

### Requirement: Known recognition hallucinations are discarded
The system SHALL discard a transcript that consists only of a known recognition hallucination (subtitle credits, subscription calls, blank-audio markers, a phrase repeated in a loop, the word "you" alone or repeated, ignoring case and punctuation) instead of pasting it. When a transcript holds spoken words as well as a known hallucination phrase or marker, the system SHALL remove the sentence holding the hallucination phrase and the marker, and SHALL keep every other word of the transcript.

#### Scenario: Silence produces a subtitle credit
- **WHEN** the user records silence and the service returns a subtitle credit line
- **THEN** nothing is pasted

#### Scenario: Silence produces a lone "you"
- **GIVEN** a recording of silence lasting 1, 10 or 60 seconds
- **AND** the service detects the spoken language automatically
- **WHEN** the user transcribes the recording
- **THEN** no text is produced

#### Scenario: Speech containing "you" is kept
- **WHEN** the service returns a transcript in which "you" appears together with other words
- **THEN** the transcript is passed on for pasting

#### Scenario: Hallucination after speech is removed and the speech is kept
- **GIVEN** the service returns a transcript of spoken sentences followed by a subscription call such as "Thanks for watching."
- **WHEN** the user finishes the recording
- **THEN** the spoken sentences are passed on for pasting
- **AND** the subscription call is not

#### Scenario: Marker inside speech is removed and the speech is kept
- **GIVEN** the service returns a transcript of spoken words with a blank-audio or music marker among them
- **WHEN** the user finishes the recording
- **THEN** the spoken words are passed on for pasting, without the marker

### Requirement: Dropped connection to the service is retried
The system SHALL send the recording again, up to 3 more times with a growing pause of at most 1.5 seconds in total, when the speech recognition service refuses or drops the connection before it answers, whatever connection error the operating system reports for the drop, and SHALL log each dropped attempt; each new attempt SHALL go to another copy of the service when one is up. Only when every attempt fails is the cycle dropped as a service failure. An unusable answer SHALL NOT be retried; a timeout or an error answer is retried as its own requirement states.

#### Scenario: Service is briefly busy
- **GIVEN** the speech recognition service drops the first two connections and then answers normally
- **WHEN** the user finishes a recording
- **THEN** the transcript the service returns is passed on for pasting, and each of the two dropped attempts is logged

#### Scenario: Drop reported as an invalid argument
- **GIVEN** the speech recognition service drops the connection after the recording was sent
- **AND** the operating system reports the drop as an invalid argument instead of a reset
- **WHEN** the user finishes a recording
- **THEN** the recording is sent again, as for any dropped connection

#### Scenario: Service keeps dropping the connection
- **GIVEN** the speech recognition service drops every connection
- **WHEN** the user finishes a recording
- **THEN** the recording is sent 4 times in total and then the cycle is dropped as a service failure

#### Scenario: Service answers with an error
- **GIVEN** the speech recognition service answers every request with an error
- **WHEN** the user finishes a recording
- **THEN** the recording is sent 3 times in total and then the cycle is dropped as a service failure

#### Scenario: A chunk whose copy is killed is transcribed by another copy
- **GIVEN** the speech recognition service for dictation runs as several copies
- **WHEN** the copy that is to transcribe a chunk is killed as the chunk reaches it
- **THEN** that chunk is transcribed by another copy, and the killed copy gets no second attempt

### Requirement: No speech yields no text
The system SHALL treat an empty transcript from the speech recognition service as a recording without speech: nothing is pasted or printed, and the cycle ends without a service failure.

#### Scenario: Service hears no speech
- **GIVEN** the speech recognition service returns no text for a recording
- **WHEN** the user transcribes the recording file
- **THEN** no text is produced and the app does not report a failure

### Requirement: Recording after a long pause reaches the service at once
The system SHALL send a recording to the speech recognition service on a connection the service has not closed, whatever time passed since the previous recording, so that the first attempt reaches the service and no dropped attempt is logged.

#### Scenario: Service closed the idle connection during a long pause
- **GIVEN** a recording was transcribed
- **AND** the speech recognition service has since reset the connection that recording used
- **WHEN** the user finishes the next recording
- **THEN** its transcript is passed on for pasting, and no dropped attempt is logged
