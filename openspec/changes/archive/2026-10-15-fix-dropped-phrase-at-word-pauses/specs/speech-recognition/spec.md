## MODIFIED Requirements

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
