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
The system SHALL drop the current recording, log the failure and stay ready for the next recording when the speech recognition service fails, times out or returns an unusable answer.

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
The system SHALL discard a transcript that consists only of a known recognition hallucination (subtitle credits, subscription calls, blank-audio markers, a phrase repeated in a loop) instead of pasting it.

#### Scenario: Silence produces a subtitle credit
- **WHEN** the user records silence and the service returns a subtitle credit line
- **THEN** nothing is pasted

### Requirement: Long recordings are transcribed in chunks
The system SHALL split a dictation or a recording file longer than the chunk threshold into overlapping chunks, transcribe each through the service, and merge the results without duplicating words at the seams, even when the service transcribes the overlapping audio differently in the two chunks. Where two chunks share no words, the merge SHALL keep every word of both.

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

### Requirement: Outdated speech recognition settings are reported
The system SHALL refuse a configuration that still uses the removed in-process recognition settings and SHALL name the setting that replaces each one.

#### Scenario: Config still points at a model file
- **WHEN** the user starts the app with a configuration that sets a speech recognition model file path
- **THEN** the app exits with a message saying the model is now chosen when starting the service and naming the service address setting

### Requirement: Dropped connection to the service is retried
The system SHALL send the recording again, up to 3 more times with a growing pause of at most 1.5 seconds in total, when the speech recognition service refuses or drops the connection before it answers, and SHALL log each dropped attempt; only when every attempt fails is the cycle dropped as a service failure. An error answer, an unusable answer or a timeout SHALL NOT be retried.

#### Scenario: Service is briefly busy
- **GIVEN** the speech recognition service drops the first two connections and then answers normally
- **WHEN** the user finishes a recording
- **THEN** the transcript the service returns is passed on for refinement and pasting, and each of the two dropped attempts is logged

#### Scenario: Service keeps dropping the connection
- **GIVEN** the speech recognition service drops every connection
- **WHEN** the user finishes a recording
- **THEN** the recording is sent 4 times in total and then the cycle is dropped as a service failure

#### Scenario: Service answers with an error
- **GIVEN** the speech recognition service answers every request with an error
- **WHEN** the user finishes a recording
- **THEN** the recording is sent once and the cycle is dropped as a service failure
