# Spec Delta

## MODIFIED Requirements

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
