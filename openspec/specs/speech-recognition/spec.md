# speech-recognition Specification

## Purpose

Turns the speech the user recorded into a raw transcript by delegating recognition to a speech recognition service that runs on the same machine, so the app never runs a recognition model itself.

## Requirements

### Requirement: Transcription through a local speech recognition service
The system SHALL obtain every raw transcript from the configured speech recognition service using the OpenAI Audio Transcriptions API, and SHALL NOT load a speech recognition model into its own process.

#### Scenario: Recording is transcribed by the service
- **WHEN** the user finishes a recording while the speech recognition service is running
- **THEN** the transcript the service returns is passed on for refinement and pasting

#### Scenario: App starts without a local model file
- **WHEN** the user starts the app with no speech recognition model on disk and the service running
- **THEN** the app starts and is ready to record

### Requirement: Speech recognition service stays on this machine
The system SHALL accept only a speech recognition service address that resolves to this machine, so recorded audio never leaves it.

#### Scenario: Remote service address is rejected
- **WHEN** the user starts the app with a speech recognition service address on another host
- **THEN** the app exits before recording anything and explains that only a local service is allowed

### Requirement: Unavailable speech recognition service is reported at startup
The system SHALL check that the speech recognition service is reachable before it accepts the first recording, and SHALL exit with a message that names the service address when it is not.

#### Scenario: Service is not running at startup
- **WHEN** the user starts the app while no speech recognition service is listening at the configured address
- **THEN** the app exits within 3 seconds with a message naming the address and how to start the service

### Requirement: Service failure during a cycle drops only that cycle
The system SHALL drop the current recording, log the failure and stay ready for the next recording when the speech recognition service fails, times out or returns an unusable answer on every attempt the retry rules allow.

#### Scenario: Service stops between recordings
- **WHEN** the user finishes a recording after the speech recognition service has stopped
- **THEN** nothing is pasted, the failure is logged, and the app accepts the next recording

### Requirement: Spoken language follows the configuration
The system SHALL send the configured spoken language with every request; the value `auto` SHALL make the service detect the language and still return a transcript.

#### Scenario: Automatic language detection
- **WHEN** the configured language is `auto` and the user records Russian speech
- **THEN** the raw transcript contains the Russian words that were spoken

#### Scenario: Fixed language
- **WHEN** the configured language is `en` and the user records English speech
- **THEN** the raw transcript contains the English words that were spoken

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
- **THEN** the transcript is passed on for refinement and pasting

#### Scenario: Hallucination after speech is removed and the speech is kept
- **GIVEN** the service returns a transcript of spoken sentences followed by a subscription call such as "Thanks for watching."
- **WHEN** the user finishes the recording
- **THEN** the spoken sentences are passed on for refinement and pasting
- **AND** the subscription call is not

#### Scenario: Marker inside speech is removed and the speech is kept
- **GIVEN** the service returns a transcript of spoken words with a blank-audio or music marker among them
- **WHEN** the user finishes the recording
- **THEN** the spoken words are passed on for refinement and pasting, without the marker

### Requirement: Long recordings are transcribed in chunks
The system SHALL split a dictation or a recording file longer than the chunk threshold into overlapping chunks, transcribe each through the service, and merge the results without duplicating words at the seams, even when the service transcribes the overlapping audio differently in the two chunks. Where two chunks share no words, the merge SHALL keep every word of both. A chunk SHALL end in a pause of the speech when the seconds before the chunk threshold hold one, judged against the recording's own background level. No chunk, its overlap included, SHALL be longer than the chunk threshold.

#### Scenario: Dictation longer than the chunk threshold
- **WHEN** the user dictates for longer than the chunk threshold
- **THEN** the raw transcript contains every spoken word once, in order

#### Scenario: Chunks transcribe the overlap differently
- **GIVEN** a dictation longer than the chunk threshold, in Russian or in English
- **AND** the service transcribes the overlapping audio differently in the two chunks: other punctuation or case, another spelling, a word cut short at the chunk edge, or a different reading of a word
- **WHEN** the dictation is transcribed
- **THEN** the overlapping words appear once in the raw transcript
- **AND** the words before and after the overlap are kept

#### Scenario: Chunks share no words
- **GIVEN** a dictation longer than the chunk threshold whose chunk transcripts share no words at the seam
- **WHEN** the dictation is transcribed
- **THEN** the raw transcript contains every word of every chunk, in order

#### Scenario: Recording file longer than the chunk threshold
- **GIVEN** a recording file of Russian or English speech longer than the chunk threshold
- **WHEN** the user transcribes the recording file
- **THEN** the recording is transcribed in chunks
- **AND** the printed text repeats no phrase of the speech and misses at most one spoken word in ten

#### Scenario: Long speech under difficult recording conditions
- **GIVEN** a recording file of Russian or English speech longer than the chunk threshold, spoken slowly or fast, with pauses between sentences or between words, with low or high background noise, through a phone-quality line, quietly, clipped, or with short audio dropouts
- **WHEN** the user transcribes the recording file
- **THEN** the printed text repeats no phrase of the speech
- **AND** it misses at most one spoken word in ten

#### Scenario: Chunk seams fall on different words
- **GIVEN** Russian or English speech longer than the chunk threshold, spoken at different rates or with pauses
- **AND** the speech starts after different lead-ins, so the chunk seams fall on different words
- **WHEN** the speech is transcribed in chunks
- **THEN** the raw transcript repeats no phrase of the speech
- **AND** the merge drops at most one in fifty spoken words beyond those the chunk transcripts already missed

#### Scenario: Chunk ends in a pause rather than in a quiet moment of a word
- **GIVEN** speech longer than the chunk threshold with a pause between two phrases a few seconds before the threshold
- **AND** brief quiet moments inside words closer to the threshold
- **WHEN** the speech is split into chunks
- **THEN** the first chunk ends inside the pause

#### Scenario: Chunk ends in the longest pause
- **GIVEN** speech longer than the chunk threshold with a short pause and, earlier, a longer pause, both a few seconds before the threshold
- **WHEN** the speech is split into chunks
- **THEN** the first chunk ends in the middle of the longer pause

#### Scenario: Pause is found under background noise
- **GIVEN** speech longer than the chunk threshold with a pause a few seconds before the threshold
- **AND** steady background noise louder than a quiet room throughout
- **WHEN** the speech is split into chunks
- **THEN** the first chunk ends inside the pause

#### Scenario: Quiet speech is not taken for a pause
- **GIVEN** speech spoken quietly, longer than the chunk threshold, with a pause a few seconds before the threshold
- **WHEN** the speech is split into chunks
- **THEN** the first chunk ends inside the pause, not inside a word

#### Scenario: Short audio dropout is not a pause
- **GIVEN** speech longer than the chunk threshold whose only silence before the threshold is a microphone dropout shorter than a pause
- **WHEN** the speech is split into chunks
- **THEN** the first chunk ends at the chunk threshold

#### Scenario: Speech without a pause ends at the threshold
- **GIVEN** speech longer than the chunk threshold with no pause in the seconds before the threshold
- **WHEN** the speech is split into chunks
- **THEN** the first chunk ends at the chunk threshold

#### Scenario: No chunk is longer than the chunk threshold
- **GIVEN** speech more than twice as long as the chunk threshold
- **WHEN** the speech is split into chunks
- **THEN** every chunk, its overlap with the previous chunk included, is at most the chunk threshold long

#### Scenario: Most seams of synthesized speech fall in pauses
- **GIVEN** Russian and English speech longer than the chunk threshold, spoken slowly, fast, with pauses, with low or high background noise, quietly, clipped or with short audio dropouts
- **AND** the speech starts after different lead-ins
- **WHEN** the speech is split into chunks
- **THEN** at least four in five chunk seams fall in a pause of the speech

### Requirement: Outdated speech recognition settings are reported
The system SHALL refuse a configuration that still uses the removed in-process recognition settings or the removed fixed silence level for chunking, and SHALL name the setting that replaces each one or say that it can be deleted.

#### Scenario: Config still points at a model file
- **WHEN** the user starts the app with a configuration that sets a speech recognition model file path
- **THEN** the app exits with a message saying the model is now chosen when starting the service and naming the service address setting

#### Scenario: Config still sets a fixed silence level
- **WHEN** the user starts the app with a configuration that sets a fixed silence level for chunking
- **THEN** the app exits with a message saying pauses now follow the recording's background level and the setting can be deleted

### Requirement: Dropped connection to the service is retried
The system SHALL send the recording again, up to 3 more times with a growing pause of at most 1.5 seconds in total, when the speech recognition service refuses or drops the connection before it answers, whatever connection error the operating system reports for the drop, and SHALL log each dropped attempt; only when every attempt fails is the cycle dropped as a service failure. An unusable answer SHALL NOT be retried; a timeout or an error answer is retried as its own requirement states.

#### Scenario: Service is briefly busy
- **GIVEN** the speech recognition service drops the first two connections and then answers normally
- **WHEN** the user finishes a recording
- **THEN** the transcript the service returns is passed on for refinement and pasting, and each of the two dropped attempts is logged

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

### Requirement: Speech after a pause is transcribed
The system SHALL transcribe the speech that follows a pause in a long dictation or recording file: no phrase of more than five consecutive spoken words SHALL be missing from the raw transcript, whatever the pauses between words or sentences and wherever the chunk seams fall. Words the service mishears are not dropped phrases.

#### Scenario: Speech with pauses between words or sentences
- **GIVEN** a recording file of Russian or English speech longer than the chunk threshold, with pauses between words or between sentences
- **WHEN** the user transcribes the recording file
- **THEN** no run of more than five consecutive spoken words is missing from the printed text

#### Scenario: Speech after pauses wherever the seams fall
- **GIVEN** Russian or English speech longer than the chunk threshold, spoken at different rates or with pauses
- **AND** the speech starts after different lead-ins, so the chunk seams fall on different words
- **WHEN** the speech is transcribed in chunks
- **THEN** no run of more than five consecutive spoken words is missing from the chunk transcripts

#### Scenario: English speech with a short pause after every word keeps every phrase
- **GIVEN** a recording file of English speech longer than the chunk threshold with a short pause after every word
- **AND** the speech starts after no lead-in, a short lead-in or a longer lead-in
- **WHEN** the user transcribes the recording file
- **THEN** no run of more than five consecutive spoken words is missing from the printed text

### Requirement: No speech yields no text
The system SHALL treat an empty transcript from the speech recognition service as a recording without speech: nothing is pasted or printed, and the cycle ends without a service failure.

#### Scenario: Service hears no speech
- **GIVEN** the speech recognition service returns no text for a recording
- **WHEN** the user transcribes the recording file without refinement
- **THEN** no text is produced and the app does not report a failure

### Requirement: Recording after a long pause reaches the service at once
The system SHALL send a recording to the speech recognition service on a connection the service has not closed, whatever time passed since the previous recording, so that the first attempt reaches the service and no dropped attempt is logged.

#### Scenario: Service closed the idle connection during a long pause
- **GIVEN** a recording was transcribed
- **AND** the speech recognition service has since reset the connection that recording used
- **WHEN** the user finishes the next recording
- **THEN** its transcript is passed on for refinement and pasting, and no dropped attempt is logged

### Requirement: Audio without speech never stops the speech recognition service
The speech recognition service SHALL answer a request whose audio holds no speech with an empty transcript and SHALL keep answering later requests, in every answer format the system asks for, including the detailed format the command spotting service uses to confirm commands.

#### Scenario: Command confirmation of an utterance without speech
- **GIVEN** the service scripts started the speech recognition service for commands
- **WHEN** the command spotting service asks it to confirm an utterance that holds no speech
- **THEN** no words are heard and the service answers the next request

### Requirement: A request that times out or gets an error answer is retried
The system SHALL send a recording or chunk again, up to 2 more times, when the speech recognition service does not answer it in time or answers with an error, and SHALL log each failed attempt with how long it took; the service SHALL stop working on a request the app gave up on.

#### Scenario: The first request of a chunk hangs
- **GIVEN** the speech recognition service never answers the first request for a chunk and answers the next one normally
- **WHEN** the user finishes the dictation
- **THEN** the chunk's text is in the pasted text, and the hung attempt is logged

#### Scenario: One error answer is retried
- **GIVEN** the speech recognition service answers the first request with an error and the next one normally
- **WHEN** the user finishes a recording
- **THEN** the transcript is pasted and the error answer is logged

### Requirement: The wait for a transcript follows the length of the audio
The system SHALL give up on one request after a time that grows with the length of the audio sent, a few seconds for a short recording and never more than the configured request timeout, so a request that hangs holds the dictation for seconds rather than a minute.

#### Scenario: A hung short chunk is given up quickly
- **GIVEN** the speech recognition service never answers any request
- **WHEN** the user finishes a 5-second recording
- **THEN** the cycle is dropped as a service failure within 30 seconds of the stop, after every attempt was logged

### Requirement: A lost chunk does not lose the rest of the dictation
The system SHALL paste the text of every chunk of a dictation that was transcribed when one chunk fails on every attempt, and SHALL log which chunk was lost.

#### Scenario: The middle chunk of a long dictation fails
- **GIVEN** a dictation long enough for three chunks
- **AND** the speech recognition service fails every attempt for the second chunk
- **WHEN** the user finishes the dictation
- **THEN** the text of the first and third chunks is pasted, and the lost second chunk is logged

### Requirement: The configured prompt steers recognition
The system SHALL make the speech recognition service use the configured `[transcription] prompt` for every 30-second window of every dictation chunk, ahead of the text recognized in earlier windows of the same chunk, so domain terms named in the prompt are recognized as written there. The last piece of a chunk shorter than 5 seconds is the exception: whisper.cpp decodes it without any earlier text, the prompt included. With an empty prompt, recognition SHALL be the same as without one.

#### Scenario: A domain term named in the prompt is recognized
- **GIVEN** the configured prompt names a domain term that speech recognition misses without the prompt
- **WHEN** the user dictates a sentence with that term
- **THEN** the raw transcript contains the term as written in the prompt

#### Scenario: The prompt holds in every window of a long chunk
- **GIVEN** the configured prompt names a domain term
- **WHEN** the user dictates a chunk longer than 30 seconds that says the term after the first 30 seconds and goes on speaking after it
- **THEN** the raw transcript contains the term as written in the prompt

### Requirement: Text of one request never steers another
The system SHALL keep the text recognized for one request out of every other request. Within a chunk sent with a prompt, the text recognized in earlier windows MAY steer the next window after the prompt; without a prompt, text recognized in one window SHALL NOT be carried into the next.

#### Scenario: A dictation does not echo the previous one
- **GIVEN** the user dictated a sentence and it was transcribed
- **WHEN** the user dictates an unrelated sentence
- **THEN** the raw transcript of the second dictation contains none of the words of the first that were not spoken again

### Requirement: Dictation is recognized with the large-v3-turbo model
The service scripts SHALL start the speech recognition service for dictation — the one the app sends dictations and recording files to — with the Whisper large-v3-turbo model, 5-bit quantized, unless the user names another model when starting it, so that long speech under difficult recording conditions misses at most one spoken word in ten.

#### Scenario: Dictation is recognized with the large-v3-turbo model by default
- **GIVEN** the models that the service scripts download are on disk
- **WHEN** the user starts the services with the service scripts without naming a model
- **THEN** the speech recognition service for dictation recognizes speech with the large-v3-turbo model

#### Scenario: Dictation is recognized with the model the user names
- **GIVEN** the models that the service scripts download are on disk
- **WHEN** the user starts the services with the service scripts and names another speech recognition model for dictation
- **THEN** the speech recognition service for dictation recognizes speech with the named model
- **AND** the speech recognition service for commands keeps the small model
